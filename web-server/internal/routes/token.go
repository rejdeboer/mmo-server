package routes

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/golang-jwt/jwt"
	"github.com/rejdeboer/multiplayer-server/internal/db"
	"github.com/rejdeboer/multiplayer-server/pkg/httperrors"
	"github.com/rs/zerolog"
	"golang.org/x/crypto/bcrypt"
)

type UserCredentials struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

type TokenResponse struct {
	Token            string `json:"token"`
	ExpiresInSeconds uint16 `json:"expiresInSeconds"`
}

func (env *Env) getToken(signingKey string, tokenExpirationSeconds uint16) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		log := zerolog.Ctx(ctx)

		var credentials UserCredentials
		err := json.NewDecoder(r.Body).Decode(&credentials)
		if err != nil {
			httperrors.Write(w, err.Error(), http.StatusBadRequest)
			log.Error().Err(err).Msg("invalid payload")
			return
		}

		q := db.New(env.Pool)

		user, err := q.GetUserByEmail(ctx, credentials.Email)
		if err != nil {
			httperrors.Write(w, "invalid email or password", http.StatusUnauthorized)
			log.Error().Err(err).Str("email", credentials.Email).Msg("user with email does not exist")
			return
		}

		if !checkPasswordHash(credentials.Password, user.Passhash) {
			httperrors.Write(w, "invalid email or password", http.StatusUnauthorized)
			log.Error().Err(err).Msg("user entered wrong password")
			return
		}

		token, err := GetJwt(signingKey, tokenExpirationSeconds, user.ID.String())
		if err != nil {
			httperrors.InternalServerError(w)
			log.Error().Err(err).Msg("error signing jwt")
			return
		}

		response, err := json.Marshal(TokenResponse{
			Token:            token,
			ExpiresInSeconds: tokenExpirationSeconds,
		})
		if err != nil {
			httperrors.InternalServerError(w)
			log.Error().Err(err).Msg("error marshalling response")
			return
		}

		log.Info().Str("user_id", user.ID.String()).Msg("successful authentication")
		w.WriteHeader(http.StatusOK)
		w.Write(response)
	}
}

func GetJwt(signingKey string, tokenExpirationSeconds uint16, userId string) (string, error) {
	token := jwt.New(jwt.SigningMethodHS256)

	claims := token.Claims.(jwt.MapClaims)
	claims["user_id"] = userId
	claims["exp"] = time.Now().Add(time.Second * time.Duration(tokenExpirationSeconds)).Unix()

	tokenString, err := token.SignedString([]byte(signingKey))
	if err != nil {
		return "", err
	}

	return tokenString, nil
}

func checkPasswordHash(password, hash string) bool {
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil
}
