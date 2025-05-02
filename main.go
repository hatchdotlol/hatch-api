package main

import (
	"log"
	"net/http"

	"github.com/hatchdotlol/hatch-api/api"
)

func main() {
	r := api.Router()

	log.Printf("Starting server at :8080\n")
	http.ListenAndServe(":8080", r)
}
