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
	"github.com/hatchdotlol/hatch-api/pkg/uploads"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func UserRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureUser)
		r.Post("/", updateProfile)
	})
	r.Get("/{username}", user)
	r.Get("/{username}/pfp", userPfp)
	r.Get("/{username}/projects", userProjects)

	return r
}

func user(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "User not found", http.StatusNotFound)
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
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
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

func userPfp(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	uploads.Download(user.ProfilePicture, w, r)
}

func userProjects(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	var page int

	if _page := r.URL.Query().Get("page"); _page != "" {
		_page, err := strconv.Atoi(_page)
		if err != nil {
			sentry.CaptureException(err)
			http.Error(w, "Bad request", http.StatusBadRequest)
			return
		}
		page = _page
	} else {
		page = 0
	}

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}
	id := user.Id

	rows, err := db.Db.Query(
		"SELECT id, author, upload_ts, title, description, shared, rating, score FROM projects WHERE author = ? LIMIT ?, ?",
		id,
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
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
			http.Error(w, "Something went wrong", http.StatusInternalServerError)
			return
		}

		commentCount, err := projects.CommentCount(projectId)
		if err != nil {
			sentry.CaptureException(err)
			http.Error(w, "Something went wrong", http.StatusInternalServerError)
			return
		}

		upvotes, downvotes, err := projects.ProjectVotes(projectId)
		if err != nil {
			sentry.CaptureException(err)
			http.Error(w, "Something went wrong", http.StatusInternalServerError)
			return
		}

		projectResp = append(projectResp, models.ProjectResp{
			Id: id,
			Author: models.Author{
				Id:          user.Id,
				Username:    user.Name,
				DisplayName: user.DisplayName,
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

func updateProfile(w http.ResponseWriter, r *http.Request) {
	var form models.RegisterForm

	body := util.HttpBody(r)
	if body == nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	if err := json.Unmarshal(body, &form); err != nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}

	fmt.Fprintln(w, ":think:")
}
