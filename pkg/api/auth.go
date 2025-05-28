package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func AuthRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureVerified)
		r.Get("/me", me)
	})

	return r
}

func me(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(*users.UserRow)

	resp, _ := json.Marshal(models.UserResp{
		Id:             user.Id,
		Name:           user.Name,
		DisplayName:    user.DisplayName,
		Country:        user.Country,
		Bio:            user.Bio,
		ProfilePicture: user.ProfilePicture,
		JoinDate:       user.JoinDate,
		BannerImage:    user.BannerImage,
		Verified:       user.Verified,
		Theme:          user.Theme,
		HatchTeam:      util.Config.Mods[user.Name],
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprintln(w, string(resp))
}
