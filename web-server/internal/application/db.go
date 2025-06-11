package application

import (
	"context"
	"fmt"
	"os"

	"github.com/Azure/azure-sdk-for-go/sdk/azcore/policy"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rejdeboer/multiplayer-server/internal/configuration"
)

func GetDbConnectionPool(settings configuration.DatabaseSettings) *pgxpool.Pool {
	pool, err := pgxpool.New(context.Background(), GetDbConnectionString(settings))
	if err != nil {
		log.Error().Msg("failed to connect to db")
		panic(err)
	}

	return pool
}

func GetDbConnectionString(settings configuration.DatabaseSettings) string {
	environment := os.Getenv("ENVIRONMENT")
	if environment != "local" && environment != "" {
		settings.Password = GetDatabaseAccessToken()
	}

	dbUrl := fmt.Sprintf("postgresql://%s:%s@%s:%d/%s",
		settings.Username,
		settings.Password,
		settings.Host,
		settings.Port,
		settings.DbName,
	)

	if !settings.RequireSsl {
		dbUrl = dbUrl + "?sslmode=disable"
	}

	return dbUrl
}

func GetDatabaseAccessToken() string {
	credential := getAzureCredential()
	token, err := credential.GetToken(context.Background(), policy.TokenRequestOptions{
		Scopes: []string{"https://ossrdbms-aad.database.windows.net/.default"},
	})
	if err != nil {
		log.Fatal().Err(err).Msg("error getting database access token")
	}
	return token.Token
}
