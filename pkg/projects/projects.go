package projects

import "github.com/hatchdotlol/hatch-api/pkg/db"

type ProjectRow struct {
	Id          int64
	Author      int64
	UploadTs    int64
	Title       *string
	Description *string
	Shared      bool `json:"-"`
	Rating      string
	Score       int64
	Thumbnail   string
}

func ProjectById(id int64) (*ProjectRow, error) {
	row := db.Db.QueryRow("SELECT * FROM projects WHERE id = ?", id)

	var p ProjectRow
	if err := row.Scan(&p.Id, &p.Author, &p.UploadTs, &p.Title, &p.Description, &p.Shared, &p.Rating, &p.Score, &p.Thumbnail); err != nil {
		return nil, err
	}

	return &p, nil
}

func CommentCount(projectId int64) (*int64, error) {
	row := db.Db.QueryRow("SELECT COUNT(*) FROM comments WHERE location = 0 AND resource_id = ? AND visible = TRUE", projectId)

	var commentCount int64
	if err := row.Scan(&commentCount); err != nil {
		return nil, err
	}

	return &commentCount, nil
}

func ProjectCount(userId int64) (*int64, error) {
	row := db.Db.QueryRow("SELECT COUNT(*) FROM projects WHERE author = ?", userId)

	var projectCount int64
	if err := row.Scan(&projectCount); err != nil {
		return nil, err
	}

	return &projectCount, nil
}

func ProjectVotes(projectId int64) (*int64, *int64, error) {
	row := db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1", projectId)

	var downvotes int64
	if err := row.Scan(&downvotes); err != nil {
		return nil, nil, err
	}

	row = db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 1 AND project = ?1", projectId)

	var upvotes int64
	if err := row.Scan(&upvotes); err != nil {
		return nil, nil, err
	}

	return &upvotes, &downvotes, nil
}
