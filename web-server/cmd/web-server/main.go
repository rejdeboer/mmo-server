package main

import (
	"github.com/rejdeboer/multiplayer-server/internal/application"
	"github.com/rejdeboer/multiplayer-server/internal/configuration"
)

func main() {
	settings := configuration.ReadConfiguration("./configuration")

	app := application.Build(settings)

	err := app.Start()
	if err != nil {
		panic(err)
	}
}
