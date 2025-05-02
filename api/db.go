package api

import (
	"database/sql"
	"log"

	_ "github.com/mattn/go-sqlite3"
)

var db = CreateDB()

func CreateDB() *sql.DB {
	db, err := sql.Open("sqlite3", "./hatch.db")
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	db.Exec(`CREATE TABLE IF NOT EXISTS reports (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                reason TEXT NOT NULL,
                resource_id INTEGER NOT NULL,
                type TEXT NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS users (
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
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS auth_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS email_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY,
                author INTEGER NOT NULL,
                upload_ts INTEGER NOT NULL,
                title TEXT,
                description TEXT,
                shared INTEGER NOT NULL,
                rating TEXT NOT NULL,
                score INTEGER NOT NULL,
                thumbnail TEXT NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                author INTEGER NOT NULL,
                post_ts INTEGER NOT NULL,
                reply_to INTEGER,
                location INTEGER NOT NULL,
                resource_id INTEGER NOT NULL,
                visible INTEGER NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS ip_bans (
                id INTEGER PRIMARY KEY,
                address TEXT NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec(`CREATE TABLE IF NOT EXISTS votes (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                project INTEGER NOT NULL,
                type INTEGER NOT NULL
            )`)
	if err != nil {
		log.Fatal(err)
	}

	db.Exec("PRAGMA journal_mode=WAL")
	if err != nil {
		log.Fatal(err)
	}

	return db
}
