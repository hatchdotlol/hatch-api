package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/go-chi/chi/v5"
)

func UserRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Get("/{username}", user)

	return r
}

func user(w http.ResponseWriter, r *http.Request) {
	username := chi.URLParam(r, "username")

	row := db.QueryRow("SELECT * FROM users WHERE name = ?1 COLLATE nocase", username)
	if row != nil {
		SendError(w, NotFound)
		return
	}

	user, err := FromUserRow(row)
	if err != nil {
		SendError(w, InternalServerError)
		return
	}

	w.Header().Add("Content-Type", "application/json")
	resp, _ := json.Marshal(user)
	fmt.Fprintln(w, string(resp))
}
