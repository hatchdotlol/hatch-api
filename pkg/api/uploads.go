package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/uploads"
	"github.com/hatchdotlol/hatch-api/pkg/users"
)

func UploadRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Group(func(r chi.Router) {
		r.Use(EnsureVerified)
		r.Post("/{type:pfp|thumbnail}", upload)
		r.Post("/project", uploadProject)
	})

	return r
}

func upload(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(5e6); err != nil {
		http.Error(w, "Image exceeds 5 MB", http.StatusBadRequest)
		return
	}

	user := r.Context().Value(User).(users.User)

	file, header, err := r.FormFile("file")
	if err != nil {
		if err != http.ErrMissingFile {
			sentry.CaptureException(err)
		}

		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	defer file.Close()

	objectType := chi.URLParam(r, "type")
	bucket := "pfps"
	if objectType == "thumbnail" {
		bucket = "thumbnails"
	}

	obj, err := uploads.IngestImage(bucket, file, header, user)
	if err != nil {
		if err == uploads.ErrUnsupported {
			http.Error(w, "Unsupported file type", http.StatusBadRequest)
			return
		}

		sentry.CaptureException(err)
		http.Error(w, "Failed to upload", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, obj.Id)
}

func uploadProject(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(5e8); err != nil {
		http.Error(w, "Form exceeds 500 MB", http.StatusBadRequest)
		return
	}

	title := r.FormValue("title")
	desc := r.FormValue("description")
	if title == "" || desc == "" {
		http.Error(w, "Title/description cannot be blank", http.StatusBadRequest)
		return
	}

	user := r.Context().Value(User).(users.User)

	// ingest project
	file, header, err := r.FormFile("file")
	if err != nil {
		if err != http.ErrMissingFile {
			sentry.CaptureException(err)
		}

		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	defer file.Close()

	projectRow, err := uploads.IngestProject(file, header, user)
	if err != nil {
		if err == uploads.ErrAssetTooLarge {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		sentry.CaptureException(err)
		http.Error(w, "Failed to ingest project", http.StatusInternalServerError)
		return
	}

	// ingest thumbnail
	thumbnail, thumbHeader, err := r.FormFile("thumbnail")
	if err != nil {
		if err != http.ErrMissingFile {
			sentry.CaptureException(err)
		}

		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	defer thumbnail.Close()

	thumbRow, err := uploads.IngestImage("thumbnails", thumbnail, thumbHeader, user)
	if err != nil {
		if err == uploads.ErrUnsupported {
			http.Error(w, "Unsupported file type for thumbnail", http.StatusBadRequest)
			return
		}

		sentry.CaptureException(err)
		http.Error(w, "Failed to ingest thumbnail", http.StatusInternalServerError)
		return
	}

	p := projects.Project{
		Author:      user.Id,
		Title:       &title,
		Description: &desc,
		Thumbnail:   thumbRow.Id,
		File:        &projectRow.Id,
	}

	id, err := p.Insert()
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to ingest project", http.StatusInternalServerError)
		return
	}

	p.Id = id

	resp, _ := json.Marshal(p)
	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}
