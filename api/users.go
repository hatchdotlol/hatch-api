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
		JSONError(w, http.StatusNotFound, "User not found")
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
		JSONError(w, http.StatusInternalServerError, "Something went wrong")
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
	
	var page int
	
	if _page := r.URL.Query().Get("page"); _page != "" {
		_page, err := strconv.Atoi(_page)
		if err != nil {
			sentry.CaptureException(err)
			JSONError(w, http.StatusBadRequest, "Bad request")
			return
		}
		page = _page
	} else {
		page = 0
	}

	user, err := UserByName(username, true)
	if err != nil {
		sentry.CaptureException(err)
		JSONError(w, http.StatusNotFound, "User not found")
		return
	}
	id := user.Id

	stmt, err := db.Prepare("SELECT * FROM projects WHERE author = ? LIMIT ?, ?")
	if err != nil {
		sentry.CaptureException(err)
		JSONError(w, http.StatusInternalServerError, "Something went wrong")
		return
	}
	defer stmt.Close()

	rows, err := stmt.Query(id, page * config.perPage, (page + 1) * config.perPage)
	if err != nil {
		sentry.CaptureException(err)
		JSONError(w, http.StatusInternalServerError, "Something went wrong")
		return
	}
	defer rows.Close()

	projects := []ProjectResp{}

	for rows.Next() {
		var (
			projectId    int64
			authorId     uint32
			uploadTs     int64
			title        string
			description  string
			shared       bool
			rating       string
			score        int64
			thumbnailExt string
		)

		if err := rows.Scan(&projectId, &authorId, &uploadTs, &title, &description, &shared, &rating, &score, &thumbnailExt); err != nil {
			panic(err)
		}

		commentCount, err := CommentCount(projectId)
		if err != nil {
			sentry.CaptureException(err)
			JSONError(w, http.StatusInternalServerError, "Something went wrong")
			return
		}

		upvotes, downvotes, err := ProjectVotes(projectId)
		if err != nil {
			sentry.CaptureException(err)
			JSONError(w, http.StatusInternalServerError, "Something went wrong")
			return
		}

		thumbnail := fmt.Sprintf("/uploads/thumb/%d.%s", projectId, thumbnailExt)

		projects = append(projects, ProjectResp{
			Id: id,
			Author: Author{
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
			Thumbnail:    thumbnail,
			CommentCount: *commentCount,
			Upvotes:      *upvotes,
			Downvotes:    *downvotes,
		})
	}

	resp, _ := json.Marshal(ProjectsResp{
		Projects: projects,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}
