package api

import (
	"context"
	"database/sql"
	"os"

	_ "github.com/mattn/go-sqlite3"
)

var db *sql.DB

func InitDB(ctx context.Context) error {
	hdb, err := sql.Open("sqlite3", os.Getenv("DB_PATH"))
	if err != nil {
		return err
	}

	_, err = hdb.Exec("PRAGMA journal_mode=WAL")
	if err != nil {
		return err
	}

	tx, err := hdb.BeginTx(ctx, nil)
	if err != nil {
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS reports (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                reason TEXT NOT NULL,
                resource_id INTEGER NOT NULL,
                type TEXT NOT NULL
            )`)
	if err != nil {
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS users (
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
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS auth_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )`)
	if err != nil {
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS email_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )`)
	if err != nil {
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS projects (
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
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS comments (
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
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS ip_bans (
                id INTEGER PRIMARY KEY,
                address TEXT NOT NULL
            )`)
	if err != nil {
		return err
	}

	_, err = tx.ExecContext(ctx, `CREATE TABLE IF NOT EXISTS votes (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                project INTEGER NOT NULL,
                type INTEGER NOT NULL
            )`)
	if err != nil {
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

	stmt, err := db.Prepare(sqls)
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	row := stmt.QueryRow(name)

	user, err := UserFromRow(row)
	if err != nil {
		return nil, err
	}

	return user, nil
}

func UserByToken(token string) (*UserRow, error) {
	stmt, err := db.Prepare("SELECT * FROM users WHERE id = (SELECT user FROM auth_tokens WHERE token = ?)")
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	row := stmt.QueryRow(token)

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

func CommentCount(projectId int64) (*int64, error) {
	stmt, err := db.Prepare("SELECT COUNT(*) FROM comments WHERE location = 0 AND resource_id = ? AND visible = TRUE")
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	row := stmt.QueryRow(projectId)

	var commentCount int64
	if err := row.Scan(&commentCount); err != nil {
		return nil, err
	}

	return &commentCount, nil
}
