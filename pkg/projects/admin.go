package projects

import "github.com/hatchdotlol/hatch-api/pkg/db"

func UnshareProject(id int64, share bool) error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec("UPDATE projects SET shared = ? WHERE id = ?", share, id); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}
