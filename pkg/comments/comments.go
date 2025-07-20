package comments

import (
	"log"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
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
	Id       int64             `json:"id"`
	Content  string            `json:"content"`
	Author   models.Author     `json:"author"`
	PostDate int64             `json:"postDate"`
	ReplyTo  *int64            `json:"replyTo"`
	Replies  map[int64]Comment `json:"replies"`
}

func Comments(location Location, resource any, page int, replyTo *int64) (map[int64]Comment, error) {
	if replyTo != nil {
		log.Printf("replying to %d", *replyTo)
	}
	var replyId any
	if replyTo == nil {
		replyId = nil
	} else {
		replyId = *replyTo
	}

	rows, err := db.Db.Query(
		"SELECT id, content, author, post_ts, reply_to FROM comments WHERE location = ? AND resource_id = ? AND visible = TRUE AND reply_to IS ? LIMIT ?, ?",
		location,
		resource,
		replyId,
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	comments := make(map[int64]Comment, 0)

	for rows.Next() {
		var comment Comment
		var authorId int64

		if err := rows.Scan(&comment.Id, &comment.Content, &authorId, &comment.PostDate, &comment.ReplyTo); err != nil {
			return nil, err
		}

		author, err := users.UserFromRow(db.Db.QueryRow("SELECT * FROM users WHERE id = ?", authorId))
		if err != nil {
			return nil, err
		}

		comment.Author = models.Author{
			Id:          authorId,
			Username:    author.Name,
			DisplayName: author.DisplayName,
		}

		if replyTo == nil {
			replies, err := Comments(location, resource, 0, &comment.Id)
			if err != nil {
				return nil, err
			}

			comment.Replies = replies
		}

		comments[comment.Id] = comment
	}

	return comments, nil
}
