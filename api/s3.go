package api

import (
	"errors"
	"os"

	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"
)

var s3 *minio.Client

var ErrBadInstance = errors.New("minio instance is not configured right")

func InitS3() error {
	opts := &minio.Options{
		Creds:  credentials.NewStaticV4(os.Getenv("MINIO_ACCESS_KEY"), os.Getenv("MINIO_SECRET_KEY"), ""),
		Secure: os.Getenv("MINIO_SECURE") == "1",
	}

	var err error
	s3, err = minio.New(os.Getenv("MINIO_ENDPOINT"), opts)

	if err != nil {
		return err
	}

	return nil
}
