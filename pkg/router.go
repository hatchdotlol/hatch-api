package api

import (
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/uploads"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"github.com/rs/cors"
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
}`, util.Config.StartTime, util.Config.Version)
}

func Router() *chi.Mux {
	util.InitConfig()

	if err := sentry.Init(sentry.ClientOptions{
		Dsn: os.Getenv("SENTRY_DSN"),
	}); err != nil {
		log.Fatal(err)
	}

	if err := db.InitDB(); err != nil {
		sentry.CaptureException(err)
		log.Fatal(err)
	}

	if err := db.InitS3(); err != nil {
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

	r.Mount("/users", users.UserRouter())
	r.Mount("/projects", projects.ProjectRouter())
	r.Mount("/uploads", uploads.UploadRouter())

	return r
}
