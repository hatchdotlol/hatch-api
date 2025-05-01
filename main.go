package main

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/rs/cors"
)

var (
	startTime = time.Now().Unix()
	version   = os.Getenv("VERSION")
)

func Root(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	fmt.Fprintf(w, `{
	"startTime": "%d",
	"website": "https://hatch.lol",
	"api": "https://api.hatch.lol",
	"forums": "https://forums.hatch.lol",
	"email": "contact@hatch.lol",
	"version": "%s"
}`, startTime, version)
}

func main() {
	r := chi.NewRouter()

	cors := cors.New(cors.Options{
		AllowedOrigins:   []string{"*"},
		AllowedMethods:   []string{"GET", "POST", "OPTIONS"},
		AllowedHeaders:   []string{"*"},
		AllowCredentials: true,
	}).Handler

	r.Use(cors)
	r.Use(middleware.Recoverer)
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Logger)

	r.Options("/*", func(w http.ResponseWriter, r *http.Request) { fmt.Fprint(w, "") })
	r.Get("/", Root)

	log.Printf("Starting server at :8080\n")
	http.ListenAndServe(":8080", r)
}
