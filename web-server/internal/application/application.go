package application

import (
	"fmt"
	"net/http"

	"github.com/elastic/go-elasticsearch/v8"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rejdeboer/multiplayer-server/internal/configuration"
	"github.com/rejdeboer/multiplayer-server/internal/logger"
	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/segmentio/kafka-go"
)

var log = logger.Get()

type Application struct {
	pool     *pgxpool.Pool
	producer *kafka.Writer
	handler  http.Handler
	addr     string
}

func Build(settings configuration.Settings) Application {
	port := settings.Application.Port
	addr := fmt.Sprintf(":%d", port)

	pool := GetDbConnectionPool(settings.Database)

	handler := routes.CreateHandler(settings, &routes.Env{
		Pool:         pool,
	})

	return Application{
		addr:     addr,
		pool:     pool,
		handler:  handler,
	}
}

func (app *Application) Start() error {
	defer app.close()
	log.Info().Msg(fmt.Sprintf("Server listening on port %s", app.addr))
	return http.ListenAndServe(app.addr, app.handler)
}

func (app *Application) close() {
	app.pool.Close()
}

