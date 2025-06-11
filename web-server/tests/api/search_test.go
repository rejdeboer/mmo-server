package api

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/rejdeboer/multiplayer-server/tests/helpers"
)

func TestSearchUsers(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	testApp.InsertElasticsearch("users", routes.UserListItem{
		ID:       testUser.ID,
		Username: testUser.Username,
		Email:    testUser.Email,
	})

	t.Run("success response", func(t *testing.T) {
		path := "/user/search?query=" + testUser.ID.String()
		req, err := http.NewRequest(http.MethodGet, path, nil)
		if err != nil {
			t.Fatal(err)
		}
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))

		rr := httptest.NewRecorder()

		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 200 {
			t.Errorf("expected %d got %d", 200, status)
		}

		var response []routes.UserListItem
		err = json.NewDecoder(rr.Body).Decode(&response)
		if err != nil {
			t.Errorf("error decoding json response: %v", err)
		}

		testUserFound := false
		for _, doc := range response {
			if doc.ID == testUser.ID {
				testUserFound = true
			}
		}

		if !testUserFound {
			t.Errorf("test user %v not present in response: %v", testUser, response)
		}
	})
}
