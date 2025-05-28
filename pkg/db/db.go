package db

import (
	"database/sql"
	"os"

	_ "github.com/mattn/go-sqlite3"
)

var Db *sql.DB

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
		thumbnail TEXT NOT NULL,
		file TEXT NOT NULL
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

	Db = hdb

	return nil
}
