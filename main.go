package main

import (
	"log"
	"net/http"
	"os"
	"time"

	"github.com/getsentry/sentry-go"
	"github.com/hatchdotlol/hatch-api/pkg/api"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"github.com/joho/godotenv"
)

func main() {
	godotenv.Load()

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

	r := api.Router()

	sentry.CaptureMessage("Starting API")

	log.Printf("Starting server at :8080\n")
	http.ListenAndServe(":8080", r)

	sentry.Flush(time.Second * 5)
}
