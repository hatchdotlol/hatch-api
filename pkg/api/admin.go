package api

import (
	"fmt"
	"net/http"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
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
		sentry.CaptureMessage(fmt.Sprint(user.Name, " has deleted their account"))
	}()

	fmt.Fprint(w, "Account deletion scheduled. Please wait")
}

const banMessage = "***[%s](https://dev.hatch.lol/user?u=%s) was %sned by [%s](https://dev.hatch.lol/user?u=%s).*** 🔨"

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

	go func() {
		you := r.Context().Value(User).(*users.UserRow)
		util.LogMessage(fmt.Sprintf(banMessage, user.Name, user.Name, action, you.Name, you.Name))
	}()

	fmt.Fprintf(w, "Account %sned", action)
}
