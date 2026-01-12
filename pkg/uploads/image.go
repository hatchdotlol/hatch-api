package uploads

import (
	"errors"
	"fmt"
	"mime/multipart"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"github.com/minio/minio-go/v7"
)

var ErrUnsupported = errors.New("unsupported file type")

func ImageDimensions(imagePath string) (int, int, error) {
	out, err := exec.Command(
		"magick",
		"identify",
		"-format",
		"%w,%h",
		imagePath,
	).Output()
	if err != nil {
		return 0, 0, err
	}

	outSlice := strings.Split(string(out), ",")
	width, _ := strconv.Atoi(outSlice[0])
	height, _ := strconv.Atoi(outSlice[1])

	return width, height, nil
}

func IngestImage(bucket string, file multipart.File, header *multipart.FileHeader, user *users.User) (db.File, error) {
	id, err := util.GenerateId(16)
	if err != nil {
		return db.File{}, err
	}

	ingestDir, err := os.MkdirTemp("/tmp", "ingest")
	if err != nil {
		return db.File{}, err
	}
	defer os.RemoveAll(ingestDir)

	if err := SaveToIngest(file, ingestDir); err != nil {
		return db.File{}, err
	}

	filePath := fmt.Sprint(ingestDir, "/original")

	hash, err := FileHash(filePath)
	if err != nil {
		return db.File{}, err
	}

	f := db.File{
		Id:       id,
		Bucket:   bucket,
		Hash:     hash,
		Filename: header.Filename,
		Uploader: user.Id,
		Width:    nil,
		Height:   nil,
	}

	mime, err := exec.Command(
		"file",
		"--mime-type",
		filePath,
	).Output()
	if err != nil {
		return db.File{}, err
	}

	if !strings.HasPrefix(strings.Fields(string(mime))[1], "image/") {
		return db.File{}, ErrUnsupported
	}

	width, height, err := ImageDimensions(filePath)
	if err != nil {
		return db.File{}, err
	}

	f.Width = &width
	f.Height = &height

	frames, err := exec.Command(
		"magick",
		"identify",
		"-format",
		"%n",
		filePath,
	).Output()
	if err != nil {
		return db.File{}, err
	}

	format := "webp"
	f.Mime = "image/webp"
	if string(frames) != "1" {
		format = "gif"
		f.Mime = "image/gif"
	}

	finalPath := fmt.Sprint(ingestDir, "/.", format)

	// remove metadata, optimize and
	// resize image smallest possible axis on pfp
	desiredSize := min(*f.Width, *f.Height, 256)
	if err := exec.Command(
		"magick",
		fmt.Sprint(ingestDir, "/original"),
		"-quality",
		"90",
		"-resize",
		fmt.Sprint(desiredSize, "x", desiredSize),
		"-auto-orient",
		"-strip",
		finalPath,
	).Run(); err != nil {
		return db.File{}, err
	}

	info, err := db.Uploads.FPutObject(ctx, f.Bucket, f.Hash, finalPath, minio.PutObjectOptions{ContentType: f.Mime})
	if err != nil {
		return db.File{}, err
	}

	f.Size = &info.Size

	if err := f.Index(); err != nil {
		return db.File{}, err
	}

	tx, err := db.Db.Begin()
	if err != nil {
		return db.File{}, err
	}

	switch bucket {
	case "pfps":
		if _, err := tx.Exec(
			"UPDATE users SET profile_picture = ? WHERE id = ?",
			f.Id,
			user.Id,
		); err != nil {
			return db.File{}, err
		}
	case "thumbnails":
		if _, err := tx.Exec(
			"UPDATE projects SET thumbnail = ? WHERE id = ?",
			f.Id,
			user.Id,
		); err != nil {
			return db.File{}, err
		}
	}

	if err := tx.Commit(); err != nil {
		return db.File{}, err
	}

	return f, nil
}
