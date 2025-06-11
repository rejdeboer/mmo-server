package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/google/uuid"
	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/rejdeboer/multiplayer-server/tests/helpers"
)

func TestCreateDocument(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	documentName := "some document"

	bodyBytes, err := json.Marshal(routes.DocumentCreate{
		Name: documentName,
	})
	if err != nil {
		t.Fatal(err)
	}

	req, err := http.NewRequest(http.MethodPost, "/document", bytes.NewReader(bodyBytes))
	req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	testApp.Handler.ServeHTTP(rr, req)

	status := rr.Result().StatusCode
	if status != 200 {
		t.Errorf("expected %d got %d", 200, rr.Result().StatusCode)
	}

	var response routes.DocumentResponse
	err = json.NewDecoder(rr.Body).Decode(&response)
	if err != nil {
		t.Errorf("error decoding json response: %v", err)
	}

	if response.OwnerID != testUser.ID {
		t.Errorf("output user_id mismatch; expected %v; got %v", testUser.ID, response.ID)
	}

	if response.Name != documentName {
		t.Errorf("output name mismatch; expected %v; got %v", documentName, response.Name)
	}
}

func TestDeleteDocument(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	testDoc := testApp.GetTestDocument(testUser.ID)
	testDocID := testDoc.ID

	t.Run("success response", func(t *testing.T) {
		req, err := http.NewRequest(http.MethodDelete, "/document/"+testDocID.String(), nil)
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

	t.Run("document not found", func(t *testing.T) {
		req, err := http.NewRequest(http.MethodDelete, "/document/"+uuid.New().String(), nil)
		if err != nil {
			t.Fatal(err)
		}
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 404 {
			t.Errorf("expected %d got %d", 404, rr.Result().StatusCode)
		}
	})

	t.Run("user is not owner", func(t *testing.T) {
		req, err := http.NewRequest(http.MethodDelete, "/document/"+testDocID.String(), nil)
		if err != nil {
			t.Fatal(err)
		}

		otherUser := testApp.GetTestUser()
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(otherUser.ID))

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 404 {
			t.Errorf("expected %d got %d", 404, rr.Result().StatusCode)
		}
	})
}

func TestListDocuments(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	testDoc := testApp.GetTestDocument(testUser.ID)
	testDocID := testDoc.ID

	req, err := http.NewRequest(http.MethodGet, "/document", nil)
	req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	testApp.Handler.ServeHTTP(rr, req)

	status := rr.Result().StatusCode
	if status != 200 {
		t.Errorf("expected %d got %d", 200, rr.Result().StatusCode)
	}

	var response []routes.DocumentListItem
	err = json.NewDecoder(rr.Body).Decode(&response)
	if err != nil {
		t.Errorf("error decoding json response: %v", err)
	}

	testDocumentFound := false
	for _, doc := range response {
		if doc.ID == testDocID {
			testDocumentFound = true
		}
	}

	if !testDocumentFound {
		t.Errorf("test document %v not present in response: %v", testDocID, response)
	}
}

func TestGetDocument(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()
	testDoc := testApp.GetTestDocument(testUser.ID)
	testDocID := testDoc.ID

	t.Run("success response", func(t *testing.T) {
		req, err := http.NewRequest(http.MethodGet, "/document/"+testDocID.String(), nil)
		if err != nil {
			t.Fatal(err)
		}
		req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 200 {
			t.Errorf("expected %d got %d", 200, rr.Result().StatusCode)
		}

		var response routes.DocumentResponse
		err = json.NewDecoder(rr.Body).Decode(&response)
		if err != nil {
			t.Errorf("error decoding json response: %v", err)
		}

		if testDocID != response.ID {
			t.Errorf("documents do not match; expected: %v; was: %v", testDoc, response)
		}
	})

	t.Run("unauthorized", func(t *testing.T) {
		req, err := http.NewRequest(http.MethodGet, "/document/"+testDocID.String(), nil)
		if err != nil {
			t.Fatal(err)
		}

		rr := httptest.NewRecorder()
		testApp.Handler.ServeHTTP(rr, req)

		status := rr.Result().StatusCode
		if status != 401 {
			t.Errorf("expected %d got %d", 401, rr.Result().StatusCode)
		}
	})
}
