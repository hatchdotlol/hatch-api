package projects

import (
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
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

type Author struct {
	Id          int64   `json:"id"`
	Username    string  `json:"username"`
	DisplayName *string `json:"displayName,omitempty"`
}

type ProjectJSON struct {
	Id           int64  `json:"id"`
	Author       Author `json:"author"`
	UploadTs     int64  `json:"uploadTs"`
	Title        string `json:"title"`
	Description  string `json:"description"`
	Version      *uint  `json:"version,omitempty"`
	Rating       string `json:"rating"`
	Thumbnail    string `json:"-"`
	CommentCount int64  `json:"commentCount"`
	Upvotes      int64  `json:"upvotes"`
	Downvotes    int64  `json:"downvotes"`
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
	var commentCount int64

	err := db.Db.QueryRow("SELECT COUNT(*) FROM comments WHERE location = 0 AND resource_id = ? AND visible = TRUE", projectId).Scan(&commentCount)
	if err != nil {
		return nil, err
	}

	return &commentCount, nil
}

func ProjectCount(userId int64) (*int64, error) {
	var projectCount int64

	err := db.Db.QueryRow("SELECT COUNT(*) FROM projects WHERE author = ?", userId).Scan(&projectCount)
	if err != nil {
		return nil, err
	}

	return &projectCount, nil
}

func ProjectVotes(projectId int64) (*int64, *int64, error) {
	var downvotes int64

	err := db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1", projectId).Scan(&downvotes)
	if err != nil {
		return nil, nil, err
	}

	var upvotes int64

	err = db.Db.QueryRow("SELECT COUNT(*) FROM votes WHERE type = 1 AND project = ?1", projectId).Scan(&upvotes)
	if err != nil {
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

func ProjectInfoById(id int64) (*ProjectJSON, error) {
	p, err := ProjectById(id)
	if err != nil || !p.Shared {
		return nil, err
	}

	upv, downv, err := ProjectVotes(id)
	if err != nil {
		return nil, err
	}

	user, err := users.UserById(p.Author)
	if err != nil {
		return nil, err
	}

	commentCount, err := CommentCount(p.Id)
	if err != nil {
		return nil, err
	}

	return &ProjectJSON{
		Id: p.Id,
		Author: Author{
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

func UserProjects(user users.User, page int) (*[]ProjectJSON, error) {
	rows, err := db.Db.Query(
		"SELECT id, author, upload_ts, title, description, shared, rating, score FROM projects WHERE author = ? LIMIT ?, ?",
		user,
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	projects := []ProjectJSON{}

	for rows.Next() {
		var (
			projectId   int64
			authorId    uint32
			uploadTs    int64
			title       string
			description string
			shared      bool
			rating      string
			score       int64
		)

		if err := rows.Scan(&projectId, &authorId, &uploadTs, &title, &description, &shared, &rating, &score); err != nil {
			return nil, err
		}

		commentCount, err := CommentCount(projectId)
		if err != nil {
			return nil, err
		}

		upvotes, downvotes, err := ProjectVotes(projectId)
		if err != nil {
			return nil, err
		}

		projects = append(projects, ProjectJSON{
			Id: projectId,
			Author: Author{
				Id:          int64(authorId),
				Username:    user.Name,
				DisplayName: user.DisplayName,
			},
			UploadTs:     uploadTs,
			Title:        title,
			Description:  description,
			Version:      nil,
			Rating:       rating,
			CommentCount: *commentCount,
			Upvotes:      *upvotes,
			Downvotes:    *downvotes,
		})
	}

	return &projects, nil
}
