package api

import (
	"errors"
	"fmt"
	"mime/multipart"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"sync"
)

var ErrUnsupported = errors.New("unsupported file type")

func ImageDimensions(imagePath string) (*int, *int, error) {
	out, err := exec.Command(
		"magick",
		"identify",
		"-format",
		"%w,%h",
		imagePath,
	).Output()
	if err != nil {
		return nil, nil, err
	}

	outSlice := strings.Split(string(out), ",")
	width, _ := strconv.Atoi(outSlice[0])
	height, _ := strconv.Atoi(outSlice[1])
	
	return &width, &height, nil
}

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
		Hash:     id,
		Filename: header.Filename,
		Uploader: user.Id,
		Width:    nil,
		Height:   nil,
	}

	filePath := fmt.Sprint(ingestDir, "/original")

	mime, err := exec.Command(
		"file",
		"--mime-type",
		filePath,
	).Output()
	if err != nil {
		return nil, err
	}
	if !strings.HasPrefix("image/", strings.Fields(string(mime))[1]) {
		return nil, ErrUnsupported
	}

	width, height, err := ImageDimensions(filePath)
	if err != nil {
		return nil, err
	}

	f.Width = width
	f.Height = height

	frames, err := exec.Command(
		"magick",
		"identify",
		"-format",
		"%n",
		filePath,
	).Output()
	if err != nil {
		return nil, err
	}

	format := "webp"
	f.Mime = "image/webp"
	if string(frames) != "1" {
		format = "gif"
		f.Mime = "image/gif"
	}

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
		fmt.Sprint(ingestDir, "/.", format),
	).Run(); err != nil {
		return nil, err
	}

	var wg sync.WaitGroup

	wg.Add(1)
	go func() {
		defer wg.Done()
		
	}()

	if err := f.Index(); err != nil {
		return nil, err
	}

	return &f, nil
}
