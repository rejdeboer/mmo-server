package api

import (
	"os"
	"strings"
	"testing"

	"github.com/rejdeboer/multiplayer-server/internal/configuration"
	"github.com/rejdeboer/multiplayer-server/tests/helpers"
)

func TestMain(m *testing.M) {
	cluster := helpers.SpawnCluster()
	defer cluster.Purge()

	settings := configuration.ReadConfiguration("../../configuration")
	settings.Azure.BlobConnectionString = strings.ReplaceAll(settings.Azure.BlobConnectionString, "azurite:10000", cluster.GetAzuriteHostPort())
	settings.Application.ElasticsearchEndpoint = cluster.GetElasticsearchEndpoint()
	settings.Database.Host = "localhost"
	settings.Database.Port = cluster.GetDBPort()

	helpers.InitApplication(settings)

	code := m.Run()

	os.Exit(code)
}
