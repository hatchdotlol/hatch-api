package api

import (
	"fmt"
	"io"
	"net/http"
	"strconv"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
)

func UploadRouter() *chi.Mux {
	r := chi.NewRouter()

	r.Post("/pfp", uploadPfp)
	r.Get("/pfp/{id}", downloadPfp)

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
		JSONError(w, http.StatusInternalServerError, "Failed to upload pfp")
		return
	}

	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		JSONError(w, http.StatusInternalServerError, "Failed to upload pfp")
		return
	}

	if _, err := tx.ExecContext(
		ctx,
		"UPDATE users SET profile_picture = ? WHERE id = ?",
		fmt.Sprint("/uploads/pfp/", obj.Id),
		user.Id,
	); err != nil {
		JSONError(w, http.StatusInternalServerError, "Failed to upload pfp")
		return
	}

	if err := tx.Commit(); err != nil {
		JSONError(w, http.StatusInternalServerError, "Failed to upload pfp")
		return
	}

	fmt.Fprintln(w, obj.Hash)
}

func downloadPfp(w http.ResponseWriter, r *http.Request) {
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

	obj, info, err := GetObject("pfps", file.Hash)
	if err != nil {
		http.Error(w, "Failed to get pfp", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Disposition", fmt.Sprintf(`inline; filename=%s`, file.Filename))
	w.Header().Set("Content-Type", file.Mime)
	w.Header().Set("Content-Length", strconv.FormatInt(info.Size, 10))
	w.Header().Set("ETag", file.Id)
	w.Header().Set("Cache-Control", "public, max-age=31536000")

	_, err = io.Copy(w, obj)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to get pfp", http.StatusInternalServerError)
		return
	}
}
