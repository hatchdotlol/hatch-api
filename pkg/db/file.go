package db

import "time"

type File struct {
	Id       string
	Bucket   string
	Hash     string
	Filename string
	Mime     string
	Uploader int64
	UploadTs *int64
	Size     *int64
	Width    *int
	Height   *int
}

// Insert file into uploads index
func (f *File) Index() error {
	tx, err := Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec(
		"INSERT INTO uploads (id, bucket, hash, filename, mime, uploader, upload_ts, width, height) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
		f.Id,
		f.Bucket,
		f.Hash,
		f.Filename,
		f.Mime,
		f.Uploader,
		time.Now().Unix(),
		f.Width,
		f.Height,
	); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}

func GetFile(id string) (File, error) {
	row := Db.QueryRow("SELECT * FROM uploads WHERE id = ?", id)

	var file File
	if err := row.Scan(&file.Id, &file.Bucket, &file.Hash, &file.Filename, &file.Mime, &file.Uploader, &file.UploadTs, &file.Width, &file.Height); err != nil {
		return File{}, err
	}

	return file, nil
}
