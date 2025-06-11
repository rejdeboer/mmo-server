package middleware

import (
	"net/http"
	"time"

	"github.com/rejdeboer/multiplayer-server/internal/logger"
	"github.com/rs/zerolog/hlog"
)

func WithLogging(next http.Handler) http.Handler {
	l := logger.Get()
	hlogHandler := hlog.NewHandler(l)

	accessHandler := hlog.AccessHandler(func(r *http.Request, status, size int, duration time.Duration) {
		hlog.FromRequest(r).Info().
			Str("method", r.Method).
			Stringer("url", r.URL).
			Int("status_code", status).
			Dur("elapsed_ms", duration).
			Msg("")
	})

	userAgentHandler := hlog.UserAgentHandler("user_agent")
	requestIdHandler := hlog.RequestIDHandler("req_id", "Request-Id")

	return hlogHandler(accessHandler(userAgentHandler(requestIdHandler(next))))
}
