package uploads

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"mime/multipart"
	"os"
	"os/exec"
	"strconv"
	"strings"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/users"
)

var ErrAssetTooLarge = errors.New("project contains asset that is too large")

func IngestProject(file multipart.File, header *multipart.FileHeader, user *users.UserRow) (*db.File, error) {
	id, err := GenerateId()
	if err != nil {
		return nil, err
	}

	ingestDir, err := os.MkdirTemp("/tmp", "ingest")
	if err != nil {
		return nil, err
	}
	defer os.RemoveAll(ingestDir)

	if err := SaveToIngest(file, ingestDir); err != nil {
		return nil, err
	}

	filePath := fmt.Sprint(ingestDir, "/original")

	hash, err := FileHash(filePath)
	if err != nil {
		return nil, err
	}

	f := db.File{
		Id:       id,
		Bucket:   "projects",
		Hash:     *hash,
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
		return nil, err
	}
	if strings.Fields(string(mime))[1] != "application/zip" {
		return nil, ErrUnsupported
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
	c := 0

	// check file hash/size
	for scanner.Scan() {
		c += 1
		if c >= 1 && c <= 3 {
			continue
		}
		rows := strings.Fields(scanner.Text())
		if strings.HasPrefix(rows[0], "---") {
			break
		}
		size, _ := strconv.Atoi(rows[0])
		file := rows[2]
		fmt.Printf("%d => %s", size, file)
	}

	return &f, nil
}
