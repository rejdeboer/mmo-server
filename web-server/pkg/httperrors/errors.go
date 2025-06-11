package httperrors

import (
	"encoding/json"
	"github.com/rejdeboer/multiplayer-server/internal/logger"
	"net/http"
)

var log = logger.Get()

type Response struct {
	Message string `json:"message"`
	Status  int    `json:"status"`
}

func InternalServerError(w http.ResponseWriter) {
	Write(w, "an unexpected error occured, please try again later", http.StatusInternalServerError)
}

func Write(w http.ResponseWriter, message string, code int) {
	w.Header().Set("Content-Type", "text/json; charset=utf-8")
	w.WriteHeader(code)

	response, err := json.Marshal(Response{
		Message: message,
		Status:  code,
	})
	if err != nil {
		log.Error().Err(err).Msg("error marshalling error response")
		return
	}

	w.Write(response)
}
