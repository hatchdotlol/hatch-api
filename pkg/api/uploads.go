package api

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/db"
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
	r.Get("/{id}", download)

	return r
}

func upload(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(5e6); err != nil {
		http.Error(w, "Image exceeds 5 mb", http.StatusBadRequest)
		return
	}

	user := r.Context().Value(User).(*users.UserRow)

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
		sentry.CaptureException(err)
		http.Error(w, "Failed to upload", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, obj.Id)
}

func uploadProject(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(500e6); err != nil {
		http.Error(w, "Form exceeds 500 mb", http.StatusBadRequest)
		return
	}

	title := r.FormValue("title")
	desc := r.FormValue("description")
	if title == "" || desc == "" {
		http.Error(w, "Title/description cannot be blank", http.StatusBadRequest)
		return
	}

	user := r.Context().Value(User).(*users.UserRow)

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

	obj, err := uploads.IngestProject(file, header, user)
	if err != nil {
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

	thumbObj, err := uploads.IngestImage("thumbnails", thumbnail, thumbHeader, user)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to ingest thumbnail", http.StatusInternalServerError)
		return
	}

	p := projects.Project{
		Author:      user.Id,
		Title:       &title,
		Description: &desc,
		Thumbnail:   fmt.Sprint("/uploads/", thumbObj.Id),
		File:        fmt.Sprint("/uploads/", obj.Id),
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

func download(w http.ResponseWriter, r *http.Request) {
	file, err := db.GetFile(chi.URLParam(r, "id"))

	if err != nil {
		if err != sql.ErrNoRows {
			sentry.CaptureException(err)
		}
		http.Error(w, "Not found", http.StatusNotFound)
		return
	}

	if r.Header.Get("ETag") == file.Id || r.Header.Get("If-None-Match") == file.Id {
		w.WriteHeader(http.StatusNotModified)
		return
	}

	format := ""
	switch file.Mime {
	case "image/gif":
		format = ".gif"
	case "image/webp":
		format = ".webp"
	case "application/zip":
		format = ".sb3"
	}

	obj, info, err := uploads.GetObject(file.Bucket, file.Hash)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to get file", http.StatusInternalServerError)
		return
	}

	dispos := "attachment"
	if strings.HasPrefix(file.Mime, "image/") {
		dispos = "inline"
	}

	name := file.Id
	if format == ".sb3" {
		name = file.Filename
		format = ""
	}

	w.Header().Set("Content-Disposition", fmt.Sprintf(`%s; filename=%s%s`, dispos, name, format))
	w.Header().Set("Content-Type", file.Mime)
	w.Header().Set("Content-Length", strconv.FormatInt(info.Size, 10))
	w.Header().Set("ETag", file.Id)
	w.Header().Set("Cache-Control", "public, max-age=31536000")

	_, err = io.Copy(w, obj)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to send file", http.StatusInternalServerError)
		return
	}
}
