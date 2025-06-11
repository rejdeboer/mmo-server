package helpers

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"log"
	"net"
	"strconv"
	"time"

	"github.com/elastic/go-elasticsearch/v8"
	"github.com/elastic/go-elasticsearch/v8/typedapi/indices/create"
	"github.com/elastic/go-elasticsearch/v8/typedapi/types"
	"github.com/golang-migrate/migrate/v4"
	"github.com/golang-migrate/migrate/v4/database/postgres"
	_ "github.com/golang-migrate/migrate/v4/source/file"
	_ "github.com/jackc/pgx/v5/stdlib"
	"github.com/ory/dockertest/v3"
	"github.com/ory/dockertest/v3/docker"
	"github.com/rejdeboer/multiplayer-server/internal/application"
)

type Cluster struct {
	pool                   *dockertest.Pool
	postgresContainer      *dockertest.Resource
	azuriteContainer       *dockertest.Resource
	elasticsearchContainer *dockertest.Resource
}

func (cluster *Cluster) GetDBUrl() string {
	databaseHostAndPort := cluster.postgresContainer.GetHostPort("5432/tcp")
	return fmt.Sprintf("postgres://postgres:password@%s/multiplayer?sslmode=disable", databaseHostAndPort)
}

func (cluster *Cluster) GetDBPort() uint16 {
	port, _ := strconv.ParseUint(cluster.postgresContainer.GetPort("5432/tcp"), 0, 16)
	return uint16(port)
}

func (cluster *Cluster) GetAzuriteHostPort() string {
	return cluster.azuriteContainer.GetHostPort("10000/tcp")
}

func (cluster *Cluster) GetElasticsearchEndpoint() string {
	return "http://" + cluster.elasticsearchContainer.GetHostPort("9200/tcp")
}

func (cluster *Cluster) Purge() {
	if err := cluster.pool.Purge(cluster.postgresContainer); err != nil {
		fmt.Printf("could not purge postgres: %s", err)
	}

	if err := cluster.pool.Purge(cluster.azuriteContainer); err != nil {
		fmt.Printf("could not purge azurite: %s", err)
	}

	if err := cluster.pool.Purge(cluster.elasticsearchContainer); err != nil {
		fmt.Printf("could not purge elasticsearch: %s", err)
	}
}

func SpawnCluster() *Cluster {
	pool := createDockerPool()
	postgresContainer := createPostgresContainer(pool)
	elasticsearchContainer := createElasticsearchContainer(pool)
	azuriteContainer := createAzuriteContainer(pool)

	cluster := Cluster{
		pool:                   pool,
		postgresContainer:      postgresContainer,
		elasticsearchContainer: elasticsearchContainer,
		azuriteContainer:       azuriteContainer,
	}

	db, err := sql.Open("pgx", cluster.GetDBUrl())
	if err != nil {
		log.Fatalf("error open db connection: %s", err)
	}
	defer db.Close()

	if err := pool.Retry(waitPostgresContainerToBeReady(db)); err != nil {
		log.Fatalf("postgres container not intialized: %s", err)
	}
	startMigration(db)

	searchClient := application.GetSearchClient(cluster.GetElasticsearchEndpoint())
	if err := pool.Retry(waitElasticsearchContainerToBeReady(searchClient)); err != nil {
		log.Fatalf("elasticsearch container not intialized: %s", err)
	}
	createElasticsearchIndices(searchClient)

	if err := pool.Retry(waitAzuriteContainerToBeReady(cluster.GetAzuriteHostPort())); err != nil {
		log.Fatalf("azurite container not intialized: %s", err)
	}

	return &cluster
}

func createDockerPool() *dockertest.Pool {
	pool, err := dockertest.NewPool("")
	if err != nil {
		log.Fatalf("could not construct docker pool: %s", err)
	}

	err = pool.Client.Ping()
	if err != nil {
		log.Fatalf("could not connect to Docker: %s", err)
	}

	pool.MaxWait = 120 * time.Second
	return pool
}

func createPostgresContainer(dockerPool *dockertest.Pool) *dockertest.Resource {
	container, err := dockerPool.RunWithOptions(&dockertest.RunOptions{
		Repository: "postgres",
		Tag:        "15-alpine3.18",
		Env: []string{
			"POSTGRES_PASSWORD=password",
			"POSTGRES_USER=postgres",
			"POSTGRES_DB=multiplayer",
			"listen_addresses = '*'",
		},
	}, func(config *docker.HostConfig) {
		config.AutoRemove = true
		config.RestartPolicy = docker.RestartPolicy{Name: "no"}
	})
	if err != nil {
		log.Fatalf("could not start postgres: %s", err)
	}

	container.Expire(120)
	return container
}

func createAzuriteContainer(dockerPool *dockertest.Pool) *dockertest.Resource {
	container, err := dockerPool.RunWithOptions(&dockertest.RunOptions{
		Repository: "mcr.microsoft.com/azure-storage/azurite",
		Tag:        "latest",
	}, func(config *docker.HostConfig) {
		config.AutoRemove = true
		config.RestartPolicy = docker.RestartPolicy{Name: "no"}
	})
	if err != nil {
		log.Fatalf("could not start azurite: %s", err)
	}

	container.Expire(120)
	return container
}

func createElasticsearchContainer(dockerPool *dockertest.Pool) *dockertest.Resource {
	container, err := dockerPool.RunWithOptions(&dockertest.RunOptions{
		Repository: "docker.elastic.co/elasticsearch/elasticsearch",
		Tag:        "8.13.4",
		Env: []string{
			"xpack.security.enabled=false",
			"discovery.type=single-node",
			"ES_JAVA_OPTS=-Xms512m -Xmx512m",
		},
	}, func(config *docker.HostConfig) {
		config.AutoRemove = true
		config.RestartPolicy = docker.RestartPolicy{Name: "no"}
	})
	if err != nil {
		log.Fatalf("could not start elasticsearch: %s", err)
	}

	container.Expire(120)
	return container
}

func waitPostgresContainerToBeReady(db *sql.DB) func() error {
	return func() error {
		return db.Ping()
	}
}

func waitAzuriteContainerToBeReady(address string) func() error {
	return func() error {
		_, err := net.Dial("tcp", address)
		return err
	}
}

func waitElasticsearchContainerToBeReady(searchClient *elasticsearch.TypedClient) func() error {
	return func() error {
		isReady, err := searchClient.Ping().Do(context.Background())
		if err != nil {
			return err
		}
		if !isReady {
			return errors.New("elasticsearch is not ready yet")
		}
		return nil
	}
}

func startMigration(db *sql.DB) {
	driver, err := postgres.WithInstance(db, &postgres.Config{})
	if err != nil {
		log.Fatalf("could not init driver: %s", err)
	}

	migrate, err := migrate.NewWithDatabaseInstance(
		"file://../../db/migrations",
		"pgx", driver)
	if err != nil {
		log.Fatalf("could not apply the migration: %s", err)
	}

	migrate.Up()
}

func createElasticsearchIndices(searchClient *elasticsearch.TypedClient) {
	_, err := searchClient.Indices.Create("users").
		Request(&create.Request{
			Mappings: &types.TypeMapping{
				Properties: map[string]types.Property{
					"id":       types.NewTextProperty(),
					"username": types.NewTextProperty(),
					"email":    types.NewTextProperty(),
				},
			},
		}).
		Do(nil)
	if err != nil {
		log.Fatal("error creating users index in elasticsearch")
	}
}
