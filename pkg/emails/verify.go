package emails

import (
	"database/sql"
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

func SendVerificationEmail(name, email string) error {
	token, err := util.GenerateId(16)
	if err != nil {
		return err
	}

	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec(
		"INSERT INTO email_tokens (token, expiration_ts, user) VALUES (?, ?, ?)",
		token,
		time.Now().Add(time.Minute*30).Unix(),
		name,
	); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	if err := SendEmail("verify", name, email, token); err != nil {
		return err
	}

	return nil
}

func VerificationEmailSent(name string) (bool, error) {
	var expirationTs int64
	if err := db.Db.QueryRow("SELECT expiration_ts FROM email_tokens WHERE user = ?", name).Scan(&expirationTs); err != nil {
		if err == sql.ErrNoRows {
			return false, nil
		} else {
			return false, err
		}
	}

	if time.Now().Unix() < expirationTs {
		return true, nil
	}

	// this token has expired, delete it
	tx, err := db.Db.Begin()
	if err != nil {
		return false, err
	}

	if _, err := tx.Exec(
		"DELETE FROM email_tokens WHERE expiration_ts = ?",
		expirationTs,
	); err != nil {
		return false, err
	}

	if err := tx.Commit(); err != nil {
		return false, err
	}

	return false, nil
}
