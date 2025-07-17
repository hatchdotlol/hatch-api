package uploads

import (
	"database/sql"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/hatchdotlol/hatch-api/pkg/db"
)

func Download(id string, w http.ResponseWriter, r *http.Request) {
	file, err := db.GetFile(id)

	if err != nil {
		sentry.CaptureException(err)
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

	obj, info, err := GetObject(file.Bucket, file.Hash)
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
