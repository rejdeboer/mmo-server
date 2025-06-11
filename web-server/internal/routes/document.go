package routes

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"slices"

	"github.com/google/uuid"
	"github.com/rejdeboer/multiplayer-server/internal/db"
	"github.com/rejdeboer/multiplayer-server/pkg/httperrors"
	"github.com/rs/zerolog"
)

type DocumentCreate struct {
	Name string `json:"name"`
}

type DocumentResponse struct {
	ID           uuid.UUID   `json:"id"`
	Name         string      `json:"name"`
	OwnerID      uuid.UUID   `json:"ownerId"`
	Contributors []uuid.UUID `json:"contributors"`
}

type DocumentListItem struct {
	ID   uuid.UUID `json:"id"`
	Name string    `json:"name"`
}

func (env *Env) createDocument(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	log := zerolog.Ctx(ctx)

	var document DocumentCreate
	err := json.NewDecoder(r.Body).Decode(&document)
	if err != nil {
		httperrors.Write(w, err.Error(), http.StatusBadRequest)
		log.Error().Err(err).Msg("invalid body for create document")
		return
	}

	q := db.New(env.Pool)

	userID, err := uuid.Parse(ctx.Value("user_id").(string))
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to parse uuid")
		return
	}

	createdDocument, err := q.CreateDocument(ctx, db.CreateDocumentParams{
		Name:    document.Name,
		OwnerID: userID,
	})
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to push document to db")
		return
	}
	*log = log.With().
		Str("document_id", createdDocument.ID.String()).
		Str("user_id", userID.String()).
		Logger()
	log.Info().Msg("created new document")

	err = q.CreateDocumentContributor(ctx, db.CreateDocumentContributorParams{
		DocumentID: createdDocument.ID,
		UserID:     userID,
	})
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("error adding owner as contributor")
		return
	}
	log.Info().Msg("added owner as contributor")

	response, err := json.Marshal(DocumentResponse{
		ID:      createdDocument.ID,
		Name:    createdDocument.Name,
		OwnerID: createdDocument.OwnerID,
	})
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("error marshalling response")
		return
	}

	w.WriteHeader(http.StatusOK)
	w.Write(response)
}

func (env *Env) deleteDocument(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	log := zerolog.Ctx(ctx)

	docID, err := uuid.Parse(r.PathValue("id"))
	if err != nil {
		httperrors.Write(w, err.Error(), http.StatusBadRequest)
		log.Error().Err(err).Msg("invalid UUID provided")
		return
	}

	userID, err := uuid.Parse(ctx.Value("user_id").(string))
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to parse uuid")
		return
	}

	*log = log.With().
		Str("document_id", docID.String()).
		Str("user_id", userID.String()).
		Logger()

	q := db.New(env.Pool)

	document, err := q.GetDocumnetByID(ctx, docID)
	if err != nil {
		httperrors.Write(w, "Document not found", http.StatusNotFound)
		log.Error().Err(err).Msg("document not found")
		return
	}

	if document.OwnerID != userID {
		httperrors.Write(w, "Document not found", http.StatusNotFound)
		log.Error().Err(err).Msg("user has no right to delete document")
		return
	}

	err = q.DeleteDocument(ctx, docID)
	if err != nil {
		httperrors.Write(w, "document not found", http.StatusNotFound)
		log.Error().Err(err).Msg("failed to delete document")
		return
	}
	log.Info().Msg("deleted document")

	w.WriteHeader(http.StatusAccepted)
}

func (env *Env) listDocuments(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	log := zerolog.Ctx(ctx)

	q := db.New(env.Pool)

	userID, err := uuid.Parse(ctx.Value("user_id").(string))
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to parse uuid")
		return
	}

	dbDocuments, err := q.GetDocumentsByOwnerID(ctx, userID)
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to fetch documents from db")
		return
	}

	documents := []DocumentListItem{}
	for _, document := range dbDocuments {
		documents = append(documents, DocumentListItem{
			ID:   document.ID,
			Name: document.Name,
		})
	}

	response, err := json.Marshal(documents)
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("error marshalling response")
		return
	}

	log.Info().Int("items", len(documents)).Msg("sending document list")
	w.WriteHeader(http.StatusOK)
	w.Write(response)
}

func (env *Env) getDocument(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	log := zerolog.Ctx(ctx)

	q := db.New(env.Pool)

	docID, err := uuid.Parse(r.PathValue("id"))
	if err != nil {
		httperrors.Write(w, "Invalid document id, please use uuid format", http.StatusBadRequest)
		log.Error().Err(err).Msg("user used invalid document id format")
		return
	}

	userID, err := uuid.Parse(ctx.Value("user_id").(string))
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("failed to parse uuid")
		return
	}

	document, err := getDocumentAsUser(ctx, docID, userID, q)
	if err != nil {
		log.Error().Err(err).Msg("error fetching document")
		httperrors.Write(w, "Document not found", http.StatusNotFound)
		return
	}

	response, err := json.Marshal(document)
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("error marshalling response")
		return
	}

	log.Info().Str("document_id", docID.String()).Msg("sending document")
	w.WriteHeader(http.StatusOK)
	w.Write(response)
}

func getDocumentAsUser(
	ctx context.Context,
	docID uuid.UUID,
	userID uuid.UUID,
	q *db.Queries,
) (DocumentResponse, error) {
	rows, err := q.GetDocumentWithContributorsByID(ctx, docID)
	if err != nil {
		return DocumentResponse{}, err
	}

	var contributors []uuid.UUID
	for _, row := range rows {
		contributors = append(contributors, row.ContributorID)
	}

	if !slices.Contains(contributors, userID) {
		return DocumentResponse{}, errors.New("user does not have access rights")
	}

	return DocumentResponse{
		ID:           rows[0].ID,
		OwnerID:      rows[0].OwnerID,
		Name:         rows[0].Name,
		Contributors: contributors,
	}, nil
}
