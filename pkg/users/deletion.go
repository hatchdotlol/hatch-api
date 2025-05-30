package users

import (
	"github.com/hatchdotlol/hatch-api/pkg/db"
)

func ScheduleDeletion(user int64) error {
	rows, err := db.Db.Query("SELECT id FROM uploads WHERE uploads = ?", user)
	if err != nil {
		return err
	}
	defer rows.Close()

	uploads := []int64{}

	for rows.Next() {
		var id int64
		if err := rows.Scan(&id); err != nil {
			return err
		}
		uploads = append(uploads, id)
	}

	return nil
}
