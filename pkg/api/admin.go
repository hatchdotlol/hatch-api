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

	r.Group(func(r chi.Router) {
		r.Use(EnsureMod)
		r.Post("/users/{username}/delete", deleteUser)
		r.Post("/users/{username}/{action:ban|unban}", banUser)
	})

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

func banUser(w http.ResponseWriter, r *http.Request) {
	user, err := users.UserByName(chi.URLParam(r, "username"), true)
	if err != nil {
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	action := chi.URLParam(r, "action")

	if err := users.BanUser(user.Id, action == "ban"); err != nil {
		sentry.CaptureException(err)
		http.Error(w, fmt.Sprintf("Failed to %s user", action), http.StatusInternalServerError)
		return
	}

	fmt.Fprintf(w, "Account %sned", action)
}
