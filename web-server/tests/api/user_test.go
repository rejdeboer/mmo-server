package api

import (
	"bytes"
	"encoding/json"
	"io"
	"mime/multipart"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/google/uuid"
	"github.com/rejdeboer/multiplayer-server/internal/routes"
	"github.com/rejdeboer/multiplayer-server/pkg/httperrors"
	"github.com/rejdeboer/multiplayer-server/tests/helpers"
)

func TestCreateUser(t *testing.T) {
	cases := []struct {
		name             string
		input            routes.UserCreate
		outputStatusCode int
		outputBody       interface{}
	}{
		{
			name:             "success response",
			outputStatusCode: 200,
			outputBody: routes.UserResponse{
				ID:       uuid.New(),
				Email:    "rick.deboer@live.nl",
				Username: "rejdeboer",
			},
			input: routes.UserCreate{
				Email:    "rick.deboer@live.nl",
				Username: "rejdeboer",
				Password: "Very$ecret1",
			},
		},
		{
			name:             "missing email address",
			outputStatusCode: 400,
			outputBody: httperrors.Response{
				Message: "invalid email address",
				Status:  400,
			},
			input: routes.UserCreate{
				Email:    "",
				Username: "rejdeboer",
				Password: "Very$ecret1",
			},
		},
		{
			name:             "invalid email address",
			outputStatusCode: 400,
			outputBody: httperrors.Response{
				Message: "invalid email address",
				Status:  400,
			},
			input: routes.UserCreate{
				Email:    "rick.deboerlive.nl",
				Username: "rejdeboer",
				Password: "Very$ecret1",
			},
		},
		{
			name:             "username with special character",
			outputStatusCode: 400,
			outputBody: httperrors.Response{
				Message: "username can not contain any special characters",
				Status:  400,
			},
			input: routes.UserCreate{
				Email:    "rick.deboer@live.nl",
				Username: "rejdeboer$",
				Password: "Very$ecret1",
			},
		},
		{
			name:             "password without digits",
			outputStatusCode: 400,
			outputBody: httperrors.Response{
				Message: "password must contain at least one digit",
				Status:  400,
			},
			input: routes.UserCreate{
				Email:    "rick.deboer@live.nl",
				Username: "rejdeboer",
				Password: "Very$ecret",
			},
		},
		{
			name:             "password without special characters",
			outputStatusCode: 400,
			outputBody: httperrors.Response{
				Message: "password must contain at least one special character",
				Status:  400,
			},
			input: routes.UserCreate{
				Email:    "rick.deboer@live.nl",
				Username: "rejdeboer",
				Password: "Verysecret1",
			},
		},
	}

	testApp := helpers.GetTestApp()
	for _, testCase := range cases {
		t.Run(testCase.name, func(t *testing.T) {
			bodyBytes, err := json.Marshal(testCase.input)
			if err != nil {
				t.Fatal(err)
			}

			req, err := http.NewRequest(http.MethodPost, "/user", bytes.NewReader(bodyBytes))
			if err != nil {
				t.Fatal(err)
			}

			rr := httptest.NewRecorder()

			testApp.Handler.ServeHTTP(rr, req)

			status := rr.Result().StatusCode
			if status != testCase.outputStatusCode {
				t.Errorf("expected %d got %d", testCase.outputStatusCode, rr.Result().StatusCode)
			}

			if status != 200 {
				var response httperrors.Response
				err = json.NewDecoder(rr.Body).Decode(&response)
				if err != nil {
					t.Errorf("error decoding json response: %v", err)
				}
				if response != testCase.outputBody {
					t.Errorf("output body mismatch; expected %v; got %v", testCase.outputBody, response)
				}
				return
			}

			var response routes.UserResponse
			err = json.NewDecoder(rr.Body).Decode(&response)
			if err != nil {
				t.Errorf("error decoding json response: %v", err)
			}

			response.ID = testCase.outputBody.(routes.UserResponse).ID
			if response != testCase.outputBody {
				t.Errorf("output body mismatch; expected %v; got %v", testCase.outputBody, response)
			}
		})
	}
}

func TestUpdateUserImage(t *testing.T) {
	testApp := helpers.GetTestApp()
	testUser := testApp.GetTestUser()

	body := new(bytes.Buffer)
	writer := multipart.NewWriter(body)

	dataPart, err := writer.CreateFormFile("file", "file.png")
	if err != nil {
		t.Fatal(err)
	}

	f, err := os.Open("../resources/user-image.png")
	if err != nil {
		t.Fatal(err)
	}
	_, err = io.Copy(dataPart, f)
	if err != nil {
		t.Fatal(err)
	}
	writer.Close()

	req, err := http.NewRequest(http.MethodPost, "/user/image", body)
	req.Header.Add("Authorization", "Bearer "+testApp.GetSignedJwt(testUser.ID))
	req.Header.Set("Content-Type", writer.FormDataContentType())
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	testApp.Handler.ServeHTTP(rr, req)

	status := rr.Result().StatusCode
	if status != 200 {
		t.Errorf("expected %d got %d", 200, rr.Result().StatusCode)
	}
}
