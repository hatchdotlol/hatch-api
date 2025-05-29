package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/uploads"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func ProjectRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{id}", project)
	r.Get("/{id}/thumbnail", projectThumbnail)

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
		util.JSONError(w, http.StatusBadRequest, "Bad request")
		return
	}
	id := int64(id_)

	p, err := projects.ProjectById(id)
	if err != nil || !p.Shared {
		util.JSONError(w, http.StatusNotFound, "Project not found")
	}

	upv, downv, err := projects.ProjectVotes(id)
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	user, err := users.UserFromRow(db.Db.QueryRow("SELECT * FROM users WHERE id = ?", p.Author))
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	commentCount, err := projects.CommentCount(p.Id)
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	resp, _ := json.Marshal(models.ProjectResp{
		Id: p.Id,
		Author: models.Author{
			Id:             user.Id,
			Username:       user.Name,
			ProfilePicture: user.ProfilePicture,
			DisplayName:    user.DisplayName,
		},
		UploadTs:     *p.UploadTs,
		Title:        *p.Title,
		Description:  *p.Description,
		Version:      nil,
		Rating:       p.Rating,
		CommentCount: *commentCount,
		Upvotes:      *upv,
		Downvotes:    *downv,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}

func projectThumbnail(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	p, err := projects.ProjectById(id)
	if err != nil || !p.Shared {
		http.Error(w, "Project not found", http.StatusNotFound)
		return
	}

	uploads.Download(p.Thumbnail, w, r)
}

func projectContent(w http.ResponseWriter, r *http.Request) {
	id_, err := strconv.Atoi(chi.URLParam(r, "id"))
	if err != nil {
		http.Error(w, "Bad request", http.StatusBadRequest)
		return
	}
	id := int64(id_)

	user := r.Context().Value(User).(*users.UserRow)

	p, err := projects.ProjectById(id)
	if err != nil || !p.Shared {
		http.Error(w, "Project not found", http.StatusNotFound)
		return
	}

	if user == nil && p.Rating == "13+" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	uploads.Download(p.File, w, r)
}
