package api

import (
	"fmt"
	"log"
	"net/http"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
)

func UploadRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Post("/pfp", uploadPfp)
	r.Get("/pfp/{username}", downloadPfp)

	return r
}

func uploadPfp(w http.ResponseWriter, r *http.Request) {
	user, err := UserByToken(r.Header.Get("Authorization"))
	if err != nil {
		sentry.CaptureException(err)
		JSONError(w, http.StatusUnauthorized, "Invalid/missing token")
		return
	}

	file, header, err := r.FormFile("file")
	if err != nil {
		if err != http.ErrMissingFile {
			sentry.CaptureException(err)
		}
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	defer file.Close()

	obj, err := IngestPfp(file, header, user)
	if err != nil {
		log.Print(err)
		JSONError(w, http.StatusInternalServerError, "Failed to upload pfp")
		return
	}

	fmt.Fprintln(w, obj.Hash)
}

func downloadPfp(w http.ResponseWriter, r *http.Request) {
	// if r.Header.Get("ETag") == f.Id || r.Header.Get("If-None-Match") == f.Id {
	// 	w.WriteHeader(http.StatusNotModified)
	// 	return
	// }

	// if _size == "" || err != nil {
	// 	obj, objInfo, err := GetObject("pfps", "default.png")
	// 	if err != nil {
	// 		http.Error(w, "Failed to get object", http.StatusInternalServerError)
	// 	}

	// 	return
	// }

	// fmt.Fprintln(w, size)
}
