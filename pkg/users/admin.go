package users

import "github.com/hatchdotlol/hatch-api/pkg/db"

func BanUser(user int64, ban bool) error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec("UPDATE users SET banned = ? WHERE id = ?", ban, user); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}

func VerifyUser(user int64, verify bool) error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec("UPDATE users SET checkmark = ? WHERE id = ?", verify, user); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}
