package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func ProjectRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{id}", project)

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

	fmt.Printf("p: %v\n", p)
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
		Thumbnail:    p.Thumbnail,
		CommentCount: *commentCount,
		Upvotes:      *upv,
		Downvotes:    *downv,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}
