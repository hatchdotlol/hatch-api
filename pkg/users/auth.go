package users

import (
	"database/sql"
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func GetOrCreateToken(user int64) (*string, error) {
	// send token if exists
	authToken := db.Db.QueryRow("SELECT token FROM auth_tokens WHERE user = ?", user)

	var token string
	err := authToken.Scan(&token)

	if err != nil && err != sql.ErrNoRows {
		return nil, err
	}
	if err == sql.ErrNoRows {
		return &token, nil
	}

	tx, err := db.Db.Begin()
	if err != nil {
		return nil, err
	}

	// create new auth token to expire in 1 week
	newToken, err := util.GenerateId(20)
	if err != nil {
		return nil, err
	}

	_, err = tx.Exec(
		"INSERT INTO auth_tokens (user, token, expiration_ts) VALUES (?, ?, ?)",
		user,
		newToken,
		time.Now().Add(time.Duration(604800*time.Second)).Unix(),
	)
	if err != nil {
		return nil, err
	}

	if err := tx.Commit(); err != nil {
		return nil, err
	}

	return &newToken, nil
}

func RemoveTokens(user int64) error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec(
		"DELETE FROM auth_tokens WHERE user = ?", user,
	); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}
