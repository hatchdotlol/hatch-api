package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func UserRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{username}", user)
	r.Get("/{username}/projects", userProjects)

	return r
}

func user(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		util.JSONError(w, http.StatusNotFound, "User not found")
		return
	}

	highlightedProjects := []int64{}
	if user.HighlightedProjects != nil {
		h := strings.Split(*user.HighlightedProjects, ",")
		for _, project := range h {
			p, _ := strconv.Atoi(project)
			highlightedProjects = append(highlightedProjects, int64(p))
		}
	}

	projectCount, err := projects.ProjectCount(user.Id)
	if err != nil {
		sentry.CaptureException(err)
		util.JSONError(w, http.StatusInternalServerError, "Something went wrong")
	}

	var followerCount = 0
	if user.Followers != nil {
		followerCount = len(strings.Split(*user.Followers, ","))
	}

	var followingCount = 0
	if user.Following != nil {
		followerCount = len(strings.Split(*user.Following, ","))
	}

	resp, _ := json.Marshal(models.UserResp{
		Id:                  user.Id,
		Name:                user.Name,
		DisplayName:         user.DisplayName,
		Country:             user.Country,
		Bio:                 user.Bio,
		HighlightedProjects: highlightedProjects,
		ProfilePicture:      user.ProfilePicture,
		JoinDate:            user.JoinDate,
		BannerImage:         user.BannerImage,
		FollowerCount:       followerCount,
		FollowingCount:      followingCount,
		Verified:            user.Verified,
		Theme:               user.Theme,
		ProjectCount:        *projectCount,
		HatchTeam:           util.Config.Mods[user.Name],
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}

func userProjects(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	var page int

	if _page := r.URL.Query().Get("page"); _page != "" {
		_page, err := strconv.Atoi(_page)
		if err != nil {
			sentry.CaptureException(err)
			util.JSONError(w, http.StatusBadRequest, "Bad request")
			return
		}
		page = _page
	} else {
		page = 0
	}

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		util.JSONError(w, http.StatusNotFound, "User not found")
		return
	}
	id := user.Id
	fmt.Printf("id: %v\n", id)

	rows, err := db.Db.Query("SELECT * FROM projects WHERE author = ? LIMIT ?, ?", id, page*util.Config.PerPage, (page+1)*util.Config.PerPage)
	if err != nil {
		sentry.CaptureException(err)
		util.JSONError(w, http.StatusInternalServerError, "Something went wrong")
		return
	}
	defer rows.Close()

	projectResp := []models.ProjectResp{}

	for rows.Next() {
		var (
			projectId   int64
			authorId    uint32
			uploadTs    int64
			title       string
			description string
			shared      bool
			rating      string
			score       int64
		)

		if err := rows.Scan(&projectId, &authorId, &uploadTs, &title, &description, &shared, &rating, &score); err != nil {
			sentry.CaptureException(err)
			util.JSONError(w, http.StatusInternalServerError, "Something went wrong")
			return
		}

		commentCount, err := projects.CommentCount(projectId)
		if err != nil {
			sentry.CaptureException(err)
			util.JSONError(w, http.StatusInternalServerError, "Something went wrong")
			return
		}

		upvotes, downvotes, err := projects.ProjectVotes(projectId)
		if err != nil {
			sentry.CaptureException(err)
			util.JSONError(w, http.StatusInternalServerError, "Something went wrong")
			return
		}

		projectResp = append(projectResp, models.ProjectResp{
			Id: id,
			Author: models.Author{
				Id:             user.Id,
				Username:       user.Name,
				ProfilePicture: user.ProfilePicture,
				DisplayName:    user.DisplayName,
			},
			UploadTs:     uploadTs,
			Title:        title,
			Description:  description,
			Version:      nil,
			Rating:       rating,
			CommentCount: *commentCount,
			Upvotes:      *upvotes,
			Downvotes:    *downvotes,
		})
	}

	resp, _ := json.Marshal(models.ProjectsResp{
		Projects: projectResp,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}
