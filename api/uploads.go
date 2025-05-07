package api

import (
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
)

func UploadRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Post("/{type:pfp|thumbnail}", upload)
	r.Get("/{id}", download)

	return r
}

func upload(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(5e6); err != nil {
		http.Error(w, "Form exceeds 5 mb", http.StatusBadRequest)
		return
	}

	user, err := UserByToken(r.Header.Get("Token"))
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Invalid token", http.StatusBadRequest)
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

	objectType := chi.URLParam(r, "type")
	bucket := "pfps"
	if objectType == "thumbnail" {
		bucket = "thumbnails"
	}

	obj, err := IngestObject(bucket, file, header, user)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to upload", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, obj.Id)
}

func download(w http.ResponseWriter, r *http.Request) {
	file, err := GetFile(chi.URLParam(r, "id"))
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Not found", http.StatusNotFound)
		return
	}

	if r.Header.Get("ETag") == file.Id || r.Header.Get("If-None-Match") == file.Id {
		w.WriteHeader(http.StatusNotModified)
		return
	}

	format := "webp"
	if file.Mime == "image/gif" {
		format = "gif"
	}

	obj, info, err := GetObject(file.Bucket, file.Hash)
	if err != nil {
		http.Error(w, "Failed to get file", http.StatusInternalServerError)
		return
	}

	dispos := "attachment"
	if strings.HasPrefix(file.Mime, "image/") {
		dispos = "inline"
	}

	w.Header().Set("Content-Disposition", fmt.Sprintf(`%s; filename=%s.%s`, dispos, file.Id, format))
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
