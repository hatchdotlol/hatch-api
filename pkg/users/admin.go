package users

import "github.com/hatchdotlol/hatch-api/pkg/db"

func BanUser(user int64, ban bool) error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec("UPDATE users SET banned = ? WHERE id = ?", user, ban); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}
