package users

import (
	"context"

	"github.com/getsentry/sentry-go"
	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/minio/minio-go/v7"
)

func DeleteUser(user int64) error {
	rows, err := db.Db.Query("SELECT id, buckets FROM uploads WHERE uploads = ?", user)
	if err != nil {
		return err
	}
	defer rows.Close()

	uploadIds := []string{}
	uploadBuckets := []string{}

	for rows.Next() {
		var id string
		var bucket string
		if err := rows.Scan(&id, &bucket); err != nil {
			return err
		}
		uploadIds = append(uploadIds, id)
		uploadBuckets = append(uploadBuckets, bucket)
	}

	if _, err := db.Db.Exec("DELETE FROM uploads WHERE uploader = ?", user); err != nil {
		return err
	}

	// collect all user objects into channels
	projectCh := make(chan minio.ObjectInfo)
	thumbnailCh := make(chan minio.ObjectInfo)
	pfpCh := make(chan minio.ObjectInfo)

	go func() {
		defer close(projectCh)
		defer close(thumbnailCh)
		defer close(pfpCh)

		for i := range uploadIds {
			object, err := db.Uploads.GetObject(context.Background(), uploadBuckets[i], uploadIds[i], minio.GetObjectOptions{})
			if err != nil {
				sentry.CaptureException(err)
				continue
			}
			info, err := object.Stat()
			if err != nil {
				sentry.CaptureException(err)
				continue
			}
			switch uploadBuckets[i] {
			case "pfps":
				pfpCh <- info
			case "projects":
				projectCh <- info
			case "thumbnails":
				thumbnailCh <- info
			}
		}
	}()

	opts := minio.RemoveObjectsOptions{
		GovernanceBypass: true,
	}

	for rErr := range db.Uploads.RemoveObjects(context.Background(), "projects", projectCh, opts) {
		sentry.CaptureException(&rErr)
	}
	for rErr := range db.Uploads.RemoveObjects(context.Background(), "thumbnails", thumbnailCh, opts) {
		sentry.CaptureException(&rErr)
	}
	for rErr := range db.Uploads.RemoveObjects(context.Background(), "pfps", pfpCh, opts) {
		sentry.CaptureException(&rErr)
	}

	// TODO: delete user row when all other features are done

	return nil
}
