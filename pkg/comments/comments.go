package comments

import (
	"time"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/projects"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

type Location int

const (
	Project Location = iota
	Gallery
	User
)

type Comment struct {
	Id       int64
	Content  string
	Author   int64
	PostDate int64
	ReplyTo  *int64
	Location Location
	Resource int64
	Visible  bool
}

type CommentJSON struct {
	Id         int64           `json:"id"`
	Content    string          `json:"content"`
	Author     projects.Author `json:"author"`
	PostDate   int64           `json:"postDate"`
	ReplyTo    *int64          `json:"replyTo"`
	HasReplies bool            `json:"hasReplies"`
}

func CommentById(location Location, resource any, id any) (*CommentJSON, error) {
	row := db.Db.QueryRow(
		"SELECT id, content, author, post_ts, reply_to FROM comments WHERE location = ? AND resource_id = ? AND visible = TRUE AND id = ?",
		location,
		resource,
		id,
	)

	var comment CommentJSON
	var authorId int64

	if err := row.Scan(&comment.Id, &comment.Content, &authorId, &comment.PostDate, &comment.ReplyTo); err != nil {
		return nil, err
	}

	author, err := users.UserById(authorId)
	if err != nil {
		return nil, err
	}

	comment.Author = projects.Author{
		Id:          authorId,
		Username:    author.Name,
		DisplayName: author.DisplayName,
	}
	comment.HasReplies = HasReplies(location, resource, comment.Id)

	return &comment, nil
}

// resource should be int64 but since parsing the resource is
// not important, the type doesn't matter
func Comments(location Location, resource any, page int) (map[int64]CommentJSON, error) {
	rows, err := db.Db.Query(
		"SELECT id, content, author, post_ts, reply_to FROM comments WHERE location = ? AND resource_id = ? AND visible = TRUE AND reply_to IS NULL LIMIT ?, ?",
		location,
		resource,
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	comments := make(map[int64]CommentJSON, 0)

	for rows.Next() {
		var comment CommentJSON
		var authorId int64

		if err := rows.Scan(&comment.Id, &comment.Content, &authorId, &comment.PostDate, &comment.ReplyTo); err != nil {
			return nil, err
		}

		author, err := users.UserById(authorId)
		if err != nil {
			return nil, err
		}

		comment.Author = projects.Author{
			Id:          authorId,
			Username:    author.Name,
			DisplayName: author.DisplayName,
		}
		comment.HasReplies = HasReplies(location, resource, comment.Id)

		comments[comment.Id] = comment
	}

	return comments, nil
}

func HasReplies(location Location, resource any, comment int64) bool {
	var count int64

	if err := db.Db.QueryRow("SELECT COUNT(*) FROM comments WHERE reply_to IS ?", comment).Scan(&count); err != nil {
		return false
	}

	return count > 0
}

func Replies(location Location, resource any, comment any, page int) ([]CommentJSON, error) {
	rows, err := db.Db.Query(
		"SELECT id, content, author, post_ts, reply_to FROM comments WHERE location = ? AND resource_id = ? AND visible = TRUE AND reply_to = ? LIMIT ?, ?",
		location,
		resource,
		comment,
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	replies := []CommentJSON{}

	for rows.Next() {
		var comment CommentJSON
		var authorId int64

		if err := rows.Scan(&comment.Id, &comment.Content, &authorId, &comment.PostDate, &comment.ReplyTo); err != nil {
			return nil, err
		}

		author, err := users.UserById(authorId)
		if err != nil {
			return nil, err
		}

		comment.Author = projects.Author{
			Id:          authorId,
			Username:    author.Name,
			DisplayName: author.DisplayName,
		}

		replies = append(replies, comment)
	}

	return replies, nil
}

func (c *Comment) Insert() error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec(
		"INSERT INTO comments (content, author, post_ts, reply_to, location, resource_id, visible) VALUES (?, ?, ?, ?, ?, ?, TRUE)",
		c.Content,
		c.Author,
		time.Now().Unix(),
		c.ReplyTo,
		c.Location,
		c.Resource,
		c.Visible,
	); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}

// func AddComment(location Location, resource any, replyTo *int64) error {

// 	if _, err = tx.Exec("INSERT INTO comments"); err != nil {
// 		return err
// 	}
// }
