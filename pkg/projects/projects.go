package projects

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"

	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/db"
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

	p, err := db.ProjectById(id)
	if err != nil || !p.Shared {
		util.JSONError(w, http.StatusNotFound, "Project not found")
	}

	upv, downv, err := ProjectVotes(id)
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	user, err := db.UserFromRow(db.Db.QueryRow("SELECT * FROM users WHERE id = ?", p.Author))
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	commentCount, err := db.CommentCount(p.Id)
	if err != nil {
		util.JSONError(w, http.StatusInternalServerError, "Failed to get project")
	}

	resp, _ := json.Marshal(db.ProjectResp{
		Id: p.Id,
		Author: db.Author{
			Id:             user.Id,
			Username:       user.Name,
			ProfilePicture: user.ProfilePicture,
			DisplayName:    user.DisplayName,
		},
		UploadTs:     p.UploadTs,
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

func ProjectCount(userId int64) (*int64, error) {
	row := db.Db.QueryRow("SELECT COUNT(*) FROM projects WHERE author = ?", userId)

	var projectCount int64
	if err := row.Scan(&projectCount); err != nil {
		return nil, err
	}

	return &projectCount, nil
}

func ProjectVotes(projectId int64) (*int64, *int64, error) {
	row := db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1", projectId)

	var downvotes int64
	if err := row.Scan(&downvotes); err != nil {
		return nil, nil, err
	}

	row = db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 1 AND project = ?1", projectId)

	var upvotes int64
	if err := row.Scan(&upvotes); err != nil {
		return nil, nil, err
	}

	return &upvotes, &downvotes, nil
}
