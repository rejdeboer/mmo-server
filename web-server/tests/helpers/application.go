package helpers

import (
	"context"
	"log"
	"net/http"

	"github.com/Azure/azure-sdk-for-go/sdk/storage/azblob/container"
	"github.com/brianvoe/gofakeit"
	"github.com/elastic/go-elasticsearch/v8"
	"github.com/elastic/go-elasticsearch/v8/typedapi/types/enums/refresh"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rejdeboer/multiplayer-server/internal/application"
	"github.com/rejdeboer/multiplayer-server/internal/configuration"
	"github.com/rejdeboer/multiplayer-server/internal/db"
	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/segmentio/kafka-go"
	"golang.org/x/crypto/bcrypt"
)

var app *TestApp

type TestApp struct {
	Handler      http.Handler
	SigningKey   string
	dbpool       *pgxpool.Pool
	searchClient *elasticsearch.TypedClient
}

type TestUser struct {
	ID       uuid.UUID
	Email    string
	Username string
	Password string
}

// Should be run in the main test function
func InitApplication(settings configuration.Settings) {
	producer := &kafka.Writer{
		Addr:     kafka.TCP(settings.Application.KafkaEndpoint),
		Balancer: &kafka.LeastBytes{},
	}

	dbpool := application.GetDbConnectionPool(settings.Database)

	searchClient := application.GetSearchClient(settings.Application.ElasticsearchEndpoint)

	blobClient := application.GetBlobClient(settings.Azure)
	_, err := blobClient.CreateContainer(context.Background(), routes.USER_IMAGES_CONTAINER, &container.CreateOptions{})
	if err != nil {
		log.Fatalf("error creating user images container: %v", err)
	}

	handler := routes.CreateHandler(settings, &routes.Env{
		Pool:         dbpool,
		Producer:     producer,
		Blob:         application.GetBlobClient(settings.Azure),
		SearchClient: searchClient,
	})

	app = &TestApp{
		Handler:      handler,
		SigningKey:   settings.Application.SigningKey,
		dbpool:       dbpool,
		searchClient: searchClient,
	}
}

func GetTestApp() *TestApp {
	if app == nil {
		log.Fatal("application not instantiated yet, please do so in the testing main function")
	}
	return app
}

func (app *TestApp) GetTestUser() TestUser {
	q := db.New(app.dbpool)

	password := gofakeit.Password(true, true, true, true, false, 8)
	username := gofakeit.Username()
	bytes, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		log.Fatalf("error hashing test user password: %s", err)
	}

	passhash := string(bytes)

	user, err := q.CreateUser(context.Background(), db.CreateUserParams{
		Email:    gofakeit.Email(),
		Username: username,
		Passhash: passhash,
	})
	if err != nil {
		log.Fatalf("error storing test user in db: %s", err)
	}

	return TestUser{
		ID:       user.ID,
		Email:    user.Email,
		Username: user.Username,
		Password: password,
	}
}

func (app *TestApp) GetSignedJwt(userID uuid.UUID) string {
	token, err := routes.GetJwt(
		app.SigningKey,
		10000,
		userID.String(),
	)
	if err != nil {
		log.Fatalf("error creating test token: %s", err)
	}
	return token
}

func (app *TestApp) GetTestDocument(ownerID uuid.UUID) routes.DocumentResponse {
	q := db.New(app.dbpool)

	name := gofakeit.Name()

	document, err := q.CreateDocument(context.Background(), db.CreateDocumentParams{
		OwnerID: ownerID,
		Name:    name,
	})
	if err != nil {
		log.Fatalf("error storing test document in db: %s", err)
	}

	err = q.CreateDocumentContributor(context.Background(), db.CreateDocumentContributorParams{
		DocumentID: document.ID,
		UserID:     ownerID,
	})
	if err != nil {
		log.Fatalf("error storing owner as contributor in db: %s", err)
	}

	return routes.DocumentResponse{
		ID:   document.ID,
		Name: document.Name,
	}
}

func (app *TestApp) InsertElasticsearch(index string, doc interface{}) {
	_, err := app.searchClient.Index(index).
		Request(doc).
		Refresh(refresh.True).
		Do(context.Background())
	if err != nil {
		log.Fatalf("error inserting user in index: %s", err)
	}
}
