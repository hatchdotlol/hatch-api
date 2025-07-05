package api

import (
	"fmt"
	"net/http"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/users"
)

func AdminRouter() *chi.Mux {
	r := chi.NewRouter()
	r.Post("/users/{username}/delete", deleteUser)

	return r
}

func deleteUser(w http.ResponseWriter, r *http.Request) {
	user, err := users.UserByName(chi.URLParam(r, "username"), true)
	if err != nil {
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	go func() {
		if err := users.DeleteUser(user.Id); err != nil {
			sentry.CaptureException(err)
		}
	}()

	fmt.Fprint(w, "Account deletion scheduled. Please wait")
}
