package projects

import (
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/users"
)

type Project struct {
	Id          int64   `json:"id"`
	Author      int64   `json:"-"`
	UploadTs    *int64  `json:"uploadTs,omitempty"`
	Title       *string `json:"title"`
	Description *string `json:"description"`
	Shared      bool    `json:"-"`
	Rating      string  `json:"rating,omitempty"`
	Score       int64   `json:"score,omitempty"`
	Thumbnail   string  `json:"thumbnail,omitempty"`
	File        *string `json:"file,omitempty"`
}

func ProjectById(id int64) (*Project, error) {
	row := db.Db.QueryRow("SELECT * FROM projects WHERE id = ?", id)

	var p Project
	if err := row.Scan(&p.Id, &p.Author, &p.UploadTs, &p.Title, &p.Description, &p.Shared, &p.Rating, &p.Score, &p.Thumbnail, &p.File); err != nil {
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

func (p *Project) Insert() (int64, error) {
	tx, err := db.Db.Begin()
	if err != nil {
		return -1, err
	}

	insert, err := tx.Exec(
		"INSERT INTO projects (author, upload_ts, title, description, shared, rating, score, thumbnail, file) VALUES (?, ?, ?, ?, TRUE, 'N/A', 0, ?, ?)",
		p.Author,
		time.Now().Unix(),
		p.Title,
		p.Description,
		p.Thumbnail,
		p.File,
	)
	if err != nil {
		return -1, err
	}

	if err := tx.Commit(); err != nil {
		return -1, err
	}

	return insert.LastInsertId()
}

func ProjectInfoById(id int64) (*models.ProjectResp, error) {
	p, err := ProjectById(id)
	if err != nil || !p.Shared {
		return nil, err
	}

	upv, downv, err := ProjectVotes(id)
	if err != nil {
		return nil, err
	}

	user, err := users.UserFromRow(db.Db.QueryRow("SELECT * FROM users WHERE id = ?", p.Author))
	if err != nil {
		return nil, err
	}

	commentCount, err := CommentCount(p.Id)
	if err != nil {
		return nil, err
	}

	return &models.ProjectResp{
		Id: p.Id,
		Author: models.Author{
			Id:          user.Id,
			Username:    user.Name,
			DisplayName: user.DisplayName,
		},
		UploadTs:     *p.UploadTs,
		Title:        *p.Title,
		Description:  *p.Description,
		Version:      nil,
		Rating:       p.Rating,
		CommentCount: *commentCount,
		Upvotes:      *upv,
		Downvotes:    *downv,
	}, nil
}
