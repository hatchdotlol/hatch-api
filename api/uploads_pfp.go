package api

import (
	"fmt"
	"mime/multipart"
	"os"
	"os/exec"
	"strings"
)

func IngestPfp(file multipart.File, header *multipart.FileHeader, user *UserRow) (*File, error) {
	id, err := GenerateId()
	if err != nil {
		return nil, err
	}

	ingestDir := fmt.Sprint(config.ingestDir, "/", id)
	defer os.RemoveAll(ingestDir)

	if err := os.Mkdir(ingestDir, 0700); err != nil {
		return nil, err
	}

	if err := SaveToIngest(file, ingestDir); err != nil {
		return nil, err
	}

	f := File{
		Hash: id,
		Filename: header.Filename,
		Uploader: user.Id,
		Width: nil,
		Height: nil,
	}

	filePath := fmt.Sprint(ingestDir, "/original")
		
	out, err := exec.Command(
		"file",
		"--mime-type",
		filePath,
	).Output()
	if err != nil {
		return nil, err
	}

	f.Mime = strings.Fields(string(out))[1]

	if strings.HasPrefix(f.Mime, "image/") {
		out, err = exec.Command(
			"magick",
			"identify",
			"-format",
			"%w,%h",
			fmt.Sprint(ingestDir, "/original"),
		).Output()
		if err != nil {
			return nil, err
		}
		// outSlice := strings.Split(string(out), ",")
		// tfjshdfhj
	}

	return nil, nil
}
