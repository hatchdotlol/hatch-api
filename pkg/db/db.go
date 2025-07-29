package db

import (
	"database/sql"
	"embed"
	"fmt"
	"os"

	"github.com/golang-migrate/migrate/v4"
	_ "github.com/golang-migrate/migrate/v4/database/sqlite3"
	"github.com/golang-migrate/migrate/v4/source/iofs"
	_ "github.com/mattn/go-sqlite3"
)

var Db *sql.DB

//go:embed migrations/*.sql
var migrations embed.FS

func InitDB() error {
	db, err := sql.Open("sqlite3", os.Getenv("DB_PATH"))
	if err != nil {
		return err
	}

	d, err := iofs.New(migrations, "migrations")
	if err != nil {
		return err
	}

	_, err = migrate.NewWithSourceInstance("iofs", d, fmt.Sprintf("sqlite3://./%s", os.Getenv("DB_PATH")))
	if err != nil {
		return err
	}

	if _, err := db.Exec("PRAGMA journal_mode=WAL"); err != nil {
		return err
	}

	Db = db

	return nil
}
