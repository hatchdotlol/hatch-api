package api

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/rs/cors"
)

var ctx = context.Background()

func Root(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	fmt.Fprintf(w, `{
	"startTime": "%d",
	"website": "https://hatch.lol",
	"api": "https://api.hatch.lol",
	"forums": "https://forums.hatch.lol",
	"email": "contact@hatch.lol",
	"version": "%s"
}`, config.startTime, config.version)
}

func Router() *chi.Mux {
	InitConfig()

	if err := sentry.Init(sentry.ClientOptions{
		Dsn: os.Getenv("SENTRY_DSN"),
	}); err != nil {
		log.Fatal(err)
	}

	if err := InitDB(); err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}

	if err := InitS3(); err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}

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

	r.Mount("/users", UserRouter())
	r.Mount("/projects", ProjectRouter())
	r.Mount("/uploads", UploadRouter())

	return r
}
