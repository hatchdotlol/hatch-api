package api

import (
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/minio/minio-go/v7"
)

func GetObject(bucket string, objName string) (*minio.Object, *minio.ObjectInfo, error) {
	obj, err := s3.GetObject(ctx, bucket, objName, minio.GetObjectOptions{})
	if err != nil {
		return nil, nil, err
	}

	objInfo, err := s3.StatObject(ctx, bucket, objName, minio.StatObjectOptions{})
	if err != nil {
		return nil, nil, err
	}

	return obj, &objInfo, nil
}

func SaveToIngest(obj io.Reader, dir string) error {
	dst, err := os.Create(fmt.Sprint(dir, "/original"))
	if err != nil {
		return err
	}
	defer dst.Close()

	if _, err := io.Copy(dst, obj); err != nil {
		return err
	}

	return nil
}

func GenerateId() (string, error) {
	b := make([]byte, 18)
	_, err := rand.Read(b)
	if err != nil {
		return "", err
	}

	id := base64.URLEncoding.EncodeToString(b)
	id = strings.ReplaceAll(id, "-", "a")
	id = strings.ReplaceAll(id, "_", "b")
	id = strings.ReplaceAll(id, "=", "c")

	return id, err
}
