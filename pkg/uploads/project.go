package uploads

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"mime/multipart"
	"os"
	"os/exec"
	"slices"
	"strconv"
	"strings"

	"github.com/getsentry/sentry-go"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"github.com/minio/minio-go/v7"
)

var ErrAssetTooLarge = errors.New("project contains asset that is too large")

var allowedAssetExts = []string{".png", ".jpg", ".jpeg", ".gif", ".bmp", ".mp3", ".wav", ".ogg"}

func IngestProject(file multipart.File, header *multipart.FileHeader, user *users.User) (*db.File, error) {
	id, err := util.GenerateId(16)
	if err != nil {
		return nil, err
	}

	ingestDir, err := os.MkdirTemp("/tmp", "ingest")
	if err != nil {
		return nil, err
	}

	if err := SaveToIngest(file, ingestDir); err != nil {
		return nil, err
	}

	filePath := fmt.Sprint(ingestDir, "/original")

	hash, err := FileHash(filePath)
	if err != nil {
		return nil, err
	}

	mime, err := exec.Command(
		"file",
		"--mime-type",
		filePath,
	).Output()
	if err != nil {
		return nil, err
	}
	if strings.Fields(string(mime))[1] != "application/zip" {
		return nil, ErrUnsupported
	}

	f := db.File{
		Id:       id,
		Bucket:   "projects",
		Hash:     *hash,
		Filename: header.Filename,
		Mime:     "application/zip",
		Uploader: user.Id,
		Width:    nil,
		Height:   nil,
	}

	// list project contents
	unzip := exec.Command(
		"unzip",
		"-l",
		filePath,
	)
	var out bytes.Buffer
	unzip.Stdout = &out

	if err := unzip.Run(); err != nil {
		return nil, err
	}

	scanner := bufio.NewScanner(&out)
	assets := []string{}

	// check file hash/size
	for c := 0; scanner.Scan(); c++ {
		if c >= 0 && c <= 2 {
			continue
		}

		rows := strings.Fields(scanner.Text())
		if strings.HasPrefix(rows[0], "---") {
			break
		}

		size, _ := strconv.Atoi(rows[0])
		file := rows[3]

		// ignore asset if project.json or invalid extension
		if file == "project.json" ||
			!slices.ContainsFunc(allowedAssetExts, func(e string) bool {
				return strings.HasSuffix(file, e)
			}) {
			continue
		}

		// must be <=15 mb
		if size > 15000000 {
			return nil, fmt.Errorf("project contains asset that is too large: %s", file)
		}

		assets = append(assets, file)
	}

	go func() {
		if err := uploadProject(f, filePath, assets); err != nil {
			sentry.CaptureException(err)
		}
	}()

	return &f, nil
}

// prune scratch assets from project and upload
func uploadProject(file db.File, filePath string, assets []string) error {
	defer os.RemoveAll(strings.Replace(filePath, "/original", "", 1))

	// prune assets on scratch
	pruned := []string{}
	for _, a := range assets {
		exists, err := AssetExists(a)
		if err != nil {
			continue
		}
		if exists {
			pruned = append(pruned, a)
		}
	}

	args := []string{"-d", filePath}
	args = append(args, pruned...)
	_ = exec.Command("zip", args...).Run()

	info, err := db.Uploads.FPutObject(ctx, "projects", file.Hash, filePath, minio.PutObjectOptions{ContentType: file.Mime})
	if err != nil {
		return err
	}

	file.Size = &info.Size
	if err := file.Index(); err != nil {
		return err
	}

	return nil
}
