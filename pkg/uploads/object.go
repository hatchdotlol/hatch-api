package uploads

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/minio/minio-go/v7"
)

var ctx = context.Background()

func GetObject(bucket string, objName string) (minio.Object, minio.ObjectInfo, error) {
	obj, err := db.Uploads.GetObject(ctx, bucket, objName, minio.GetObjectOptions{})
	if err != nil {
		return minio.Object{}, minio.ObjectInfo{}, err
	}

	objInfo, err := db.Uploads.StatObject(ctx, bucket, objName, minio.StatObjectOptions{})
	if err != nil {
		return minio.Object{}, minio.ObjectInfo{}, err
	}

	return *obj, objInfo, nil
}

// Saves file to specified directory under "original"
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

func FileHash(path string) (string, error) {
	out, err := exec.Command(
		"sha256sum",
		path,
	).Output()
	if err != nil {
		return "", err
	}

	return strings.Fields(string(out))[0], nil
}
