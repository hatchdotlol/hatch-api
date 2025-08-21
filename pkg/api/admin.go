package api

import (
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func AdminRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureMod)
		r.Post("/users/{username}/delete", deleteUser)
		r.Post("/users/{username}/{action:ban|unban}", banUser)
		r.Post("/users/{username}/{action:verify|unverify}", verifyUser)
		r.Post("/projects/{id}/{action:unshare|share}", unshareProject)
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

	w.WriteHeader(http.StatusOK)
}

const actionMessage = "***[%s](https://dev.hatch.lol/user?u=%s) was %s by [%s](https://dev.hatch.lol/user?u=%s).*** %s"

func banUser(w http.ResponseWriter, r *http.Request) {
	user, err := users.UserByName(chi.URLParam(r, "username"), true)
	if err != nil {
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	action := chi.URLParam(r, "action")

	if err := users.BanUser(user.Id, action == "ban"); err != nil {
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	go func() {
		you := r.Context().Value(User).(users.User)
		util.LogMessage(fmt.Sprintf(actionMessage, user.Name, user.Name, fmt.Sprint(action, "ned"), you.Name, you.Name, "🔨"))
	}()

	w.WriteHeader(http.StatusOK)
}

func verifyUser(w http.ResponseWriter, r *http.Request) {
	user, err := users.UserByName(chi.URLParam(r, "username"), true)
	if err != nil {
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	action := chi.URLParam(r, "action")

	if err := users.VerifyUser(user.Id, action == "verify"); err != nil {
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	go func() {
		you := r.Context().Value(User).(users.User)
		util.LogMessage(fmt.Sprintf(actionMessage, user.Name, user.Name, fmt.Sprint(strings.TrimSuffix(action, "y"), "ied"), you.Name, you.Name, "✅"))
	}()

	w.WriteHeader(http.StatusOK)
}

const unshareMessage = "***\"[%s](https://dev.hatch.lol/project?id=%d)\" was %sd by [%s](https://dev.hatch.lol/user?u=%s).*** 📁"

func unshareProject(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	project, err := projects.ProjectById(id)
	if err != nil {
		http.Error(w, "Project not found", http.StatusNotFound)
		return
	}

	action := chi.URLParam(r, "action")

	if err := projects.UnshareProject(project.Id, action == "share"); err != nil {
		sentry.CaptureException(err)
		http.Error(w, fmt.Sprintf("Failed to %s project", action), http.StatusInternalServerError)
		return
	}

	go func() {
		you := r.Context().Value(User).(users.User)
		util.LogMessage(fmt.Sprintf(unshareMessage, *project.Title, project.Id, action, you.Name, you.Name))
	}()

	w.WriteHeader(http.StatusOK)
}
