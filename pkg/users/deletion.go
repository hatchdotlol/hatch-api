package users

import (
	"fmt"
	"strings"

	"github.com/hatchdotlol/hatch-api/pkg/db"
)

func in(baseQuery string, values any) string {
	numValues := len(values.([]any))

	placeholders := make([]string, numValues)
	for i := range placeholders {
		placeholders[i] = "?"
	}
	inClause := strings.Join(placeholders, ",")

	modifiedQuery := strings.Replace(baseQuery, "(?)", fmt.Sprintf("(%s)", inClause), 1)

	return modifiedQuery
}

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

	query := in("DELETE FROM uploads WHERE uploader IN (?)", uploads)
	db.Db.Exec(query, uploads)

	return nil
}
