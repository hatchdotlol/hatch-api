package api

import (
	"log/slog"
	"os"

	"github.com/minio/minio-go/v7"
	"github.com/minio/minio-go/v7/pkg/credentials"
)

var s3 *minio.Client

func InitS3() error {
	opts := &minio.Options{
		Creds:  credentials.NewStaticV4(os.Getenv("MINIO_ACCESS_KEY"), os.Getenv("MINIO_SECRET_KEY"), ""),
		Secure: os.Getenv("MINIO_SECURE") == "1",
	}

	// TODO: certificate handling for secure minio

	var err error
	s3, err = minio.New(os.Getenv("MINIO_ENDPOINT"), opts)

	if err != nil {
		return err
	}

	if _, err := s3.BucketExists(ctx, "projects"); err != nil {
		slog.Warn("MinIO endpoint is not available")
	}

	return nil
}
