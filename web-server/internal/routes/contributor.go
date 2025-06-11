package routes

import (
	"encoding/json"
	"net/http"

	"github.com/google/uuid"
	"github.com/rejdeboer/multiplayer-server/internal/db"
	"github.com/rejdeboer/multiplayer-server/pkg/httperrors"
	"github.com/rs/zerolog"
)

type DocumentContributorCreate struct {
	UserID uuid.UUID `json:"userId"`
}

func (env *Env) addDocumentContributor(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	log := zerolog.Ctx(ctx)

	var contributor DocumentContributorCreate
	err := json.NewDecoder(r.Body).Decode(&contributor)
	if err != nil {
		httperrors.Write(w, err.Error(), http.StatusBadRequest)
		log.Error().Err(err).Msg("invalid body for create contributor")
		return
	}

	q := db.New(env.Pool)

	docID, err := uuid.Parse(r.PathValue("document_id"))
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

	*log = log.With().
		Str("document_id", docID.String()).
		Str("user_id", userID.String()).
		Str("contributor_id", contributor.UserID.String()).
		Logger()

	_, err = getDocumentAsUser(ctx, docID, userID, q)
	if err != nil {
		log.Error().Err(err).Msg("error fetching document")
		httperrors.Write(w, "Document not found", http.StatusNotFound)
		return
	}

	err = q.CreateDocumentContributor(ctx, db.CreateDocumentContributorParams{
		DocumentID: docID,
		UserID:     contributor.UserID,
	})
	if err != nil {
		httperrors.InternalServerError(w)
		log.Error().Err(err).Msg("error adding contributor")
	}

	log.Info().Msg("added contributor")
	w.WriteHeader(http.StatusAccepted)
}
