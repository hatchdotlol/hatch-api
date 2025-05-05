package api

import (
	"fmt"
	"log"
	"mime/multipart"
	"os"
)

type File struct {
	Id string
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
		log.Print(err)
		return nil, err
	}
	
	return &File{Id: id}, nil
}
