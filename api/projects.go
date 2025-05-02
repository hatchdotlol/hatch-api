package api

import (
	"github.com/go-chi/chi/v5"
)

func ProjectsRouter() *chi.Mux {
	r := chi.NewRouter()

	return r
}

func ProjectCount(userId int64) (*int64, error) {
	stmt, err := db.Prepare("SELECT COUNT(*) FROM projects WHERE author = ?")
	if err != nil {
		return nil, err
	}
	defer stmt.Close()

	row := stmt.QueryRow(userId)

	var projectCount int64
	if err := row.Scan(&projectCount); err != nil {
		return nil, err
	}

	return &projectCount, nil
}
