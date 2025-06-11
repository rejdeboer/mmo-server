package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/rejdeboer/multiplayer-server/tests/helpers"
)

func TestCreateContributor(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	testDoc := testApp.GetTestDocument(testUser.ID)
	testDocID := testDoc.ID

	t.Run("success response", func(t *testing.T) {
		otherUser := testApp.GetTestUser()
		bodyBytes, err := json.Marshal(routes.DocumentContributorCreate{
			UserID: otherUser.ID,
		})
		if err != nil {
			t.Fatal(err)
		}

		req, err := http.NewRequest(
			http.MethodPost,
			"/document/"+testDocID.String()+"/contributor",
			bytes.NewReader(bodyBytes),
		)
		if err != nil {
			t.Fatal(err)
		}
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 202 {
			t.Errorf("expected %d got %d", 202, rr.Result().StatusCode)
		}
	})

	t.Run("user tries to add themselves", func(t *testing.T) {
		otherUser := testApp.GetTestUser()
		bodyBytes, err := json.Marshal(routes.DocumentContributorCreate{
			UserID: otherUser.ID,
		})
		if err != nil {
			t.Fatal(err)
		}

		req, err := http.NewRequest(
			http.MethodPost,
			"/document/"+testDocID.String()+"/contributor",
			bytes.NewReader(bodyBytes),
		)
		if err != nil {
			t.Fatal(err)
		}
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(otherUser.ID))

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 404 {
			t.Errorf("expected %d got %d", 404, rr.Result().StatusCode)
		}
	})
}
