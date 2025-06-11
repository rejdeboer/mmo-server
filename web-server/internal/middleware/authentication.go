package middleware

import (
	"context"
	"net/http"
	"strings"

	"github.com/dgrijalva/jwt-go"
	"github.com/rejdeboer/multiplayer-server/pkg/httperrors"
	"github.com/rs/zerolog"
)

func WithAuth(signingKey string) func(http.HandlerFunc) http.HandlerFunc {
	return func(next http.HandlerFunc) http.HandlerFunc {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ctx := r.Context()
			log := zerolog.Ctx(ctx)

			authHeader := r.Header["Authorization"]
			if authHeader == nil || len(authHeader) == 0 {
				httperrors.Write(w, "please provide an 'Authorization' header", http.StatusUnauthorized)
				log.Error().Msg("received request without auth header")
				return
			}

			authHeaderParts := strings.Split(authHeader[0], " ")
			if len(authHeaderParts) != 2 || authHeaderParts[0] != "Bearer" {
				httperrors.Write(w, "please provide a valid bearer token in the auth header", http.StatusUnauthorized)
				log.Error().Str("header", authHeader[0]).Msg("malformed auth header")
				return
			}

			token := authHeaderParts[1]
			claims, err := verifyAndGetClaims(token, signingKey)
			if err != nil {
				httperrors.Write(w, "invalid jwt token", http.StatusUnauthorized)
				log.Error().Err(err).Msg("invalid jwt token")
				return
			}

			userID := claims["user_id"].(string)
			ctx = context.WithValue(ctx, "user_id", userID)
			*log = log.With().
				Str("user_id", userID).
				Logger()

			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

func verifyAndGetClaims(token string, verificationSecret string) (jwt.MapClaims, error) {
	claims := jwt.MapClaims{}
	_, err := jwt.ParseWithClaims(token, claims, func(t *jwt.Token) (interface{}, error) {
		return []byte(verificationSecret), nil
	})
	if err != nil {
		return nil, err
	}

	return claims, nil
}
