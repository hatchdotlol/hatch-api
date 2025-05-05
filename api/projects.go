package api

import (
	"github.com/go-chi/chi/v5"
)

func ProjectRouter() *chi.Mux {
	r := chi.NewRouter()

	return r
}

func ProjectCount(userId int64) (*int64, error) {
	row := db.QueryRow("SELECT COUNT(*) FROM projects WHERE author = ?", userId)

	var projectCount int64
	if err := row.Scan(&projectCount); err != nil {
		return nil, err
	}

	return &projectCount, nil
}

func ProjectVotes(projectId int64) (*int64, *int64, error) {
	row := db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1", projectId)

	var downvotes int64
	if err := row.Scan(&downvotes); err != nil {
		return nil, nil, err
	}

	row = db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 1 AND project = ?1", projectId)

	var upvotes int64
	if err := row.Scan(&upvotes); err != nil {
		return nil, nil, err
	}

	return &upvotes, &downvotes, nil
}
