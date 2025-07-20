package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/comments"
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
		r.Post("/{username}/{action:follow|unfollow}", followUser)
	})
	r.Get("/{username}", user)
	r.Get("/{username}/pfp", userPfp)
	r.Get("/{username}/projects", userProjects)
	r.Get("/{username}/{group:followers|following}", userPeople)
	r.Get("/{username}/comments", userComments)

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
	fmt.Fprint(w, string(resp))
}

func userPfp(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := users.UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "User not found", http.StatusNotFound)
		return
	}

	uploads.Download(&user.ProfilePicture, nil, w, r)
}

func userProjects(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	page := util.Page(r)

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

	resp, _ := json.Marshal(projectResp)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
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

	fmt.Fprint(w, ":think:")
}

func followUser(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(*users.User)
	action := chi.URLParam(r, "action")
	followee := chi.URLParam(r, "username")

	if err := users.Follow(followee, user.Id, action == "follow"); err != nil {
		sentry.CaptureException(err)
		http.Error(w, fmt.Sprintf("Failed to %s %s", action, followee), http.StatusInternalServerError)
		return
	}

	Action := "Followed"
	if action == "unfollow" {
		Action = "Unfollowed"
	}
	fmt.Fprintf(w, "%s %s", Action, followee)
}

func userPeople(w http.ResponseWriter, r *http.Request) {
	group := chi.URLParam(r, "group")

	user, err := users.UserByName(chi.URLParam(r, "username"), true)
	if err != nil {
		http.Error(w, "User not found", http.StatusNotFound)
	}

	f := user.Followers
	if group == "following" {
		f = user.Following
	}

	w.Header().Add("Content-Type", "application/json")

	if f == nil || *f == "" {
		fmt.Fprint(w, "[]")
		return
	}

	people, err := users.UsersFromIds(strings.TrimRight(*f, ","), util.Page(r))
	if err != nil {
		fmt.Print(err)
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	resp, _ := json.Marshal(people)

	fmt.Fprint(w, string(resp))
}

func userComments(w http.ResponseWriter, r *http.Request) {
	page := util.Page(r)

	comments, err := comments.Comments(comments.User, chi.URLParam(r, "id"), page, nil)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to get comments", http.StatusInternalServerError)
		return
	}

	resp, _ := json.Marshal(comments)

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}
