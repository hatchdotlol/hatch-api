package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/go-chi/chi/v5"
)

func UserRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{username}", user)

	return r
}

type UserResp struct {
	Id                  int64   `json:"id"`
	Name                string  `json:"name"`
	DisplayName         *string `json:"displayName"`
	Country             string  `json:"country"`
	Bio                 *string `json:"bio"`
	HighlightedProjects []int64 `json:"highlightedProjects"`
	ProfilePicture      string  `json:"profilePicture"`
	JoinDate            string  `json:"joinDate"`
	BannerImage         *string `json:"bannerImage"`
	FollowerCount       int     `json:"followerCount"`
	FollowingCount      int     `json:"followingCount"`
	Verified            bool    `json:"verified"`
	Theme               *string `json:"theme"`
	ProjectCount        int64   `json:"projectCount"`
	HatchTeam           bool    `json:"hatchTeam"`
}

func user(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	stmt, err := db.Prepare("SELECT * FROM users WHERE name = ? COLLATE nocase")
	if err != nil {
		SendError(w, InternalServerError)
		return
	}
	defer stmt.Close()

	row := stmt.QueryRow(username)

	if row == nil {
		SendError(w, NotFound)
		return
	}

	user, err := FromUserRow(row)
	if err != nil {
		SendError(w, NotFound)
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
		SendError(w, InternalServerError)
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
