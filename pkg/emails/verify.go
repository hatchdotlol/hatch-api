package emails

import (
	"errors"
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

var ErrVerificationSent = errors.New("verification email already sent")

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
		"INSERT INTO email_tokens (user, token, expiration_ts) VALUES (?, ?, ?)",
		name,
		token,
		time.Now().Add(time.Minute*30).Unix(),
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
