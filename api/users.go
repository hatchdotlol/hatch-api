package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
)

func UserRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{username}", user)
	r.Get("/{username}/projects", userProjects)

	return r
}

func user(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		SendError(w, http.StatusNotFound, "User not found")
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

	projectCount, err := ProjectCount(user.Id)
	if err != nil {
		sentry.CaptureException(err)
		SendError(w, http.StatusInternalServerError, "Something went wrong")
	}

	resp, _ := json.Marshal(UserResp{
		Id:                  user.Id,
		Name:                user.Name,
		DisplayName:         user.DisplayName,
		Country:             user.Country,
		Bio:                 user.Bio,
		HighlightedProjects: highlightedProjects,
		ProfilePicture:      user.ProfilePicture,
		JoinDate:            user.JoinDate,
		BannerImage:         user.BannerImage,
		FollowerCount:       len(strings.Split(user.Followers, ",")),
		FollowingCount:      len(strings.Split(user.Following, ",")),
		Verified:            user.Verified,
		Theme:               user.Theme,
		ProjectCount:        *projectCount,
		HatchTeam:           config.mods[user.Name],
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}

func userProjects(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	user, err := UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		SendError(w, http.StatusNotFound, "User not found")
		return
	}
	id := user.Id

	stmt, err := db.Prepare("SELECT * FROM projects WHERE author = ?")
	if err != nil {
		sentry.CaptureException(err)
		SendError(w, http.StatusInternalServerError, "Something went wrong")
		return
	}
	defer stmt.Close()

	rows, err := stmt.Query(id)
	if err != nil {
		sentry.CaptureException(err)
		SendError(w, http.StatusInternalServerError, "Something went wrong")
		return
	}
	defer rows.Close()
}