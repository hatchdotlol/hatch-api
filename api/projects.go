package api

import (
	"github.com/go-chi/chi/v5"
)

func ProjectRouter() *chi.Mux {
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

func ProjectVotes(projectId int64) (*int64, *int64, error) {
	stmt, err := db.Prepare("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1")
	if err != nil {
		return nil, nil, err
	}
	defer stmt.Close()

	row := stmt.QueryRow(projectId)

	var downvotes int64
	if err := row.Scan(&downvotes); err != nil {
		return nil, nil, err
	}

	stmt, err = db.Prepare("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1")
	if err != nil {
		return nil, nil, err
	}
	defer stmt.Close()

	row = stmt.QueryRow(projectId)

	var upvotes int64
	if err := row.Scan(&upvotes); err != nil {
		return nil, nil, err
	}

	return &upvotes, &downvotes, nil
}
