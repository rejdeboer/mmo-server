package application

import (
	"sync"

	"github.com/Azure/azure-sdk-for-go/sdk/azidentity"
	"github.com/Azure/azure-sdk-for-go/sdk/storage/azblob"
	"github.com/rejdeboer/multiplayer-server/internal/configuration"
)

var once sync.Once
var credential *azidentity.DefaultAzureCredential

func GetBlobClient(settings configuration.AzureSettings) *azblob.Client {

	var client *azblob.Client
	var err error
	if settings.BlobConnectionString != "" {
		// Local
		client, err = azblob.NewClientFromConnectionString(settings.BlobConnectionString, nil)
	} else {
		// Production
		client, err = azblob.NewClient(settings.BlobStorageEndpoint, getAzureCredential(), nil)
	}
	if err != nil {
		log.Fatal().Err(err).Msg("error creating azure blob storage client")
	}

	return client
}

func getAzureCredential() *azidentity.DefaultAzureCredential {
	once.Do(func() {
		c, err := azidentity.NewDefaultAzureCredential(nil)
		if err != nil {
			log.Fatal().Err(err).Msg("error getting azure credential")
		}
		credential = c
	})
	return credential
}
