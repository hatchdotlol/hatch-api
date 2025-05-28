package main

import (
	"log"
	"net/http"
	"time"

	"github.com/getsentry/sentry-go"
	api "github.com/hatchdotlol/hatch-api/pkg/api"
)

func main() {
	r := api.Router()

	sentry.CaptureMessage("Starting API")

	log.Printf("Starting server at :8080\n")
	http.ListenAndServe(":8080", r)

	sentry.Flush(time.Second * 5)
}
