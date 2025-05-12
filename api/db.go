package api

import (
	"database/sql"
	"fmt"
	"os"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

var db *sql.DB

func InitDB() error {
	hdb, err := sql.Open("sqlite3", os.Getenv("DB_PATH"))
	if err != nil {
		return err
	}

	if _, err := hdb.Exec("PRAGMA journal_mode=WAL"); err != nil {
		return err
	}

	tx, err := hdb.Begin()
	if err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS reports (
		id INTEGER PRIMARY KEY,
		user INTEGER NOT NULL,
		reason TEXT NOT NULL,
		resource_id INTEGER NOT NULL,
		type TEXT NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS users (
		id INTEGER PRIMARY KEY,
		name TEXT NOT NULL,
		pw TEXT NOT NULL,
		display_name TEXT,
		country TEXT NOT NULL,
		bio TEXT,
		highlighted_projects TEXT,
		profile_picture TEXT NOT NULL,
		join_date TEXT NOT NULL,
		banner_image TEXT,
		followers TEXT,
		following TEXT,
		verified INTEGER NOT NULL,
		email TEXT NOT NULL,
		banned INTEGER NOT NULL,
		ips TEXT NOT NULL,
		theme TEXT
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS auth_tokens (
		id INTEGER PRIMARY KEY,
		user INTEGER NOT NULL,
		token TEXT NOT NULL,
		expiration_ts INTEGER NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS email_tokens (
		id INTEGER PRIMARY KEY,
		user INTEGER NOT NULL,
		token TEXT NOT NULL,
		expiration_ts INTEGER NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS projects (
		id INTEGER PRIMARY KEY,
		author INTEGER NOT NULL,
		upload_ts INTEGER NOT NULL,
		title TEXT,
		description TEXT,
		shared INTEGER NOT NULL,
		rating TEXT NOT NULL,
		score INTEGER NOT NULL,
		thumbnail TEXT NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS comments (
		id INTEGER PRIMARY KEY,
		content TEXT NOT NULL,
		author INTEGER NOT NULL,
		post_ts INTEGER NOT NULL,
		reply_to INTEGER,
		location INTEGER NOT NULL,
		resource_id INTEGER NOT NULL,
		visible INTEGER NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS ip_bans (
		id INTEGER PRIMARY KEY,
		address TEXT NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS votes (
		id INTEGER PRIMARY KEY,
		user INTEGER NOT NULL,
		project INTEGER NOT NULL,
		type INTEGER NOT NULL
	)`); err != nil {
		return err
	}

	if _, err = tx.Exec(`CREATE TABLE IF NOT EXISTS uploads (
		id TEXT NOT NULL PRIMARY KEY,
		bucket TEXT NOT NULL,
		hash TEXT NOT NULL,
		filename TEXT NOT NULL,
		mime TEXT NOT NULL,
		uploader INTEGER NOT NULL,
		upload_ts INTEGER NOT NULL,
		width INTEGER,
		height INTEGER
	)`); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	db = hdb

	return nil
}

type UserRow struct {
	Id                  int64
	Name                string
	Pw                  string
	DisplayName         *string
	Country             string
	Bio                 *string
	HighlightedProjects *string
	ProfilePicture      string
	JoinDate            string
	BannerImage         *string
	Followers           *string
	Following           *string
	Verified            bool
	Email               string
	Banned              bool
	Ips                 string
	Theme               *string
}

func UserByName(name string, nocase bool) (*UserRow, error) {
	var sqls string
	if nocase {
		sqls = "SELECT * FROM users WHERE name = ? COLLATE nocase LIMIT 1"
	} else {
		sqls = "SELECT * FROM users WHERE name = ? LIMIT 1"
	}

	row := db.QueryRow(sqls, name)

	user, err := UserFromRow(row)
	if err != nil {
		return nil, err
	}

	return user, nil
}

func UserByToken(token string) (*UserRow, error) {
	row := db.QueryRow("SELECT * FROM users WHERE id = (SELECT user FROM auth_tokens WHERE token = ?)", token)

	user, err := UserFromRow(row)
	if err != nil {
		return nil, err
	}

	return user, nil
}

func UserFromRow(row *sql.Row) (*UserRow, error) {
	var user UserRow

	if err := row.Scan(&user.Id, &user.Name, &user.Pw, &user.DisplayName, &user.Country, &user.Bio, &user.HighlightedProjects, &user.ProfilePicture, &user.JoinDate, &user.BannerImage, &user.Followers, &user.Following, &user.Verified, &user.Email, &user.Banned, &user.Ips, &user.Theme); err != nil {
		return nil, err
	}

	return &user, nil
}

type ProjectRow struct {
	Id          int64
	Author      int64
	UploadTs    int64
	Title       *string
	Description *string
	Shared      bool `json:"-"`
	Rating      string
	Score       int64
	Thumbnail   string
}

func ProjectById(id int64) (*ProjectRow, error) {
	row := db.QueryRow("SELECT * FROM projects WHERE id = ?", id)

	var p ProjectRow
	if err := row.Scan(&p.Id, &p.Author, &p.UploadTs, &p.Title, &p.Description, &p.Shared, &p.Rating, &p.Score, &p.Thumbnail); err != nil {
		fmt.Print(err)
		return nil, err
	}

	return &p, nil
}

func CommentCount(projectId int64) (*int64, error) {
	row := db.QueryRow("SELECT COUNT(*) FROM comments WHERE location = 0 AND resource_id = ? AND visible = TRUE", projectId)

	var commentCount int64
	if err := row.Scan(&commentCount); err != nil {
		return nil, err
	}

	return &commentCount, nil
}

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
	tx, err := db.Begin()
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

func GetFile(id string) (*File, error) {
	row := db.QueryRow("SELECT * FROM uploads WHERE id = ?", id)

	var file File
	if err := row.Scan(&file.Id, &file.Bucket, &file.Hash, &file.Filename, &file.Mime, &file.Uploader, &file.UploadTs, &file.Width, &file.Height); err != nil {
		return nil, err
	}

	return &file, nil
}
