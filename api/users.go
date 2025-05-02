package api

import (
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
	fmt.Fprintf(w, "%s", username)
}
