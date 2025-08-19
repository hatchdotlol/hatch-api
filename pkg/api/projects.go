package api

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/comments"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/uploads"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func ProjectRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureUser)
		r.Post("/{id}/comments", addProjectComment)
		r.Post("/{id}/{action:upvote|downvote}", vote)
	})
	r.Get("/{id}", project)
	r.Get("/{id}/thumbnail", projectThumbnail)
	r.Get("/{id}/comments", projectComments)
	r.Get("/{id}/comments/{commentId}", projectComment)
	r.Get("/{id}/comments/{commentId}/replies", projectCommentReplies)

	// pass user context and ignore errors
	r.With(func(h http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			u, _ := users.UserByToken(r.URL.Query().Get("token"))
			ctx := context.WithValue(r.Context(), User, u)
			h.ServeHTTP(w, r.WithContext(ctx))
		})
	}).Get("/{id}/content", projectContent)

	return r
}

func project(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	project, err := projects.ProjectInfoById(id)
	if err != nil {
		if err == sql.ErrNoRows {
			http.Error(w, "Project not found", http.StatusNotFound)
			return
		} else {
			sentry.CaptureException(err)
			http.Error(w, "Failed to get project info", http.StatusInternalServerError)
			return
		}
	}
	resp, _ := json.Marshal(project)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

// TODO: fallback on thumbnail or projects stored with v1 api

func projectThumbnail(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	p, err := projects.ProjectById(id)
	if err != nil || !p.Shared {
		sentry.CaptureException(err)
		http.Error(w, "Project not found", http.StatusNotFound)
		return
	}

	uploads.Download(&p.Thumbnail, &id, w, r)
}

func projectContent(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	user := r.Context().Value(User).(users.User)

	p, err := projects.ProjectById(id)
	if err != nil || !p.Shared {
		http.Error(w, "Project not found", http.StatusNotFound)
		return
	}

	if user == nil && p.Rating == "13+" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	uploads.Download(p.File, &id, w, r)
}

func projectComment(w http.ResponseWriter, r *http.Request) {
	comment, err := comments.CommentById(comments.Project, chi.URLParam(r, "id"), chi.URLParam(r, "commentId"))
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Comment not found", http.StatusNotFound)
		return
	}

	resp, _ := json.Marshal(comment)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

func projectComments(w http.ResponseWriter, r *http.Request) {
	page := util.Page(r)

	comments, err := comments.Comments(comments.Project, chi.URLParam(r, "id"), page)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to get comments", http.StatusInternalServerError)
		return
	}

	resp, _ := json.Marshal(comments)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

func projectCommentReplies(w http.ResponseWriter, r *http.Request) {
	page := util.Page(r)

	comments, err := comments.Replies(comments.Project, chi.URLParam(r, "id"), chi.URLParam(r, "commentId"), page)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to get replies", http.StatusInternalServerError)
		return
	}

	resp, _ := json.Marshal(comments)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

func addProjectComment(w http.ResponseWriter, r *http.Request) {
	you := r.Context().Value(User).(users.User)

	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	project, err := projects.ProjectById(id)
	if err != nil {
		http.Error(w, "Project not found", http.StatusNotFound)
	}

	var form models.AddComment

	body := util.HttpBody(r)
	if body == nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	if err := json.Unmarshal(body, &form); err != nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}

	comment := comments.Comment{
		Content:  form.Content,
		Author:   you.Id,
		ReplyTo:  form.ReplyTo,
		Location: comments.Project,
		Resource: project.Id,
	}

	if err := comment.Insert(); err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to add comment", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, "Comment added")
}

func vote(w http.ResponseWriter, r *http.Request) {
	you := r.Context().Value(User).(users.User)

	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	project, err := projects.ProjectById(id)
	if err != nil {
		http.Error(w, "Project not found", http.StatusNotFound)
	}

	if err := projects.VoteProject(
		project.Id,
		you.Id,
		chi.URLParam(r, "action") == "upvote",
	); err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to vote on project", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, "Vote added")
}
