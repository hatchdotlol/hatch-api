package users

import (
	"database/sql"
	"fmt"

	"github.com/hatchdotlol/hatch-api/pkg/db"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

type User struct {
	Id                  int64
	Name                string
	Pw                  string
	DisplayName         *string
	Country             string
	Bio                 *string
	HighlightedProjects *string
	ProfilePicture      string
	JoinDate            string
	BannerImage         *string
	Followers           *string
	Following           *string
	Verified            bool
	Email               string
	Banned              bool
	Theme               *string
}

func UserByName(name string, nocase bool) (*User, error) {
	var sqls string
	if nocase {
		sqls = "SELECT * FROM users WHERE name = ? COLLATE nocase LIMIT 1"
	} else {
		sqls = "SELECT * FROM users WHERE name = ? LIMIT 1"
	}

	row := db.Db.QueryRow(sqls, name)

	user, err := UserFromRow(row)
	if err != nil {
		return nil, err
	}

	return user, nil
}

func UserByToken(token string) (*User, error) {
	row := db.Db.QueryRow("SELECT * FROM users WHERE id = (SELECT user FROM auth_tokens WHERE token = ?)", token)

	user, err := UserFromRow(row)
	if err != nil {
		return nil, err
	}

	return user, nil
}

func UserFromRow(row *sql.Row) (*User, error) {
	var user User

	if err := row.Scan(&user.Id, &user.Name, &user.Pw, &user.DisplayName, &user.Country, &user.Bio, &user.HighlightedProjects, &user.ProfilePicture, &user.JoinDate, &user.BannerImage, &user.Followers, &user.Following, &user.Verified, &user.Email, &user.Banned, &user.Theme); err != nil {
		return nil, err
	}

	return &user, nil
}

func UsersFromRows(rows *sql.Rows) (*[]User, error) {
	var users []User

	for rows.Next() {
		var user User
		if err := rows.Scan(&user.Id, &user.Name, &user.Pw, &user.DisplayName, &user.Country, &user.Bio, &user.HighlightedProjects, &user.ProfilePicture, &user.JoinDate, &user.BannerImage, &user.Followers, &user.Following, &user.Verified, &user.Email, &user.Banned, &user.Theme); err != nil {
			return nil, err
		}
		users = append(users, user)
	}

	return &users, nil
}

func (p *User) Insert() error {
	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	_, err = tx.Exec(
		`INSERT INTO users (
			name,
			pw,
			display_name,
			country,
			bio,
			highlighted_projects,
			profile_picture,
			join_date,
			banner_image,
			followers,
			following,
			verified,
			email,
			banned,
			theme
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		p.Name,
		p.Pw,
		p.DisplayName,
		p.Country,
		p.Bio,
		p.HighlightedProjects,
		p.ProfilePicture,
		p.JoinDate,
		p.BannerImage,
		p.Followers,
		p.Following,
		p.Verified,
		p.Email,
		p.Banned,
		p.Theme,
	)
	if err != nil {
		return err
	}

	return nil
}

// userString is a comma separated list of user ids
func UsersFromIds(userString string, page int) (*[]models.UserResp, error) {
	rows, err := db.Db.Query(
		fmt.Sprintf("SELECT * FROM users WHERE id in (%s) LIMIT ?, ?", userString),
		page*util.Config.PerPage,
		(page+1)*util.Config.PerPage,
	)

	if err != nil {
		return nil, err
	}
	defer rows.Close()

	users, err := UsersFromRows(rows)
	if err != nil {
		return nil, err
	}

	followers := []models.UserResp{}

	for _, f := range *users {
		followers = append(followers, models.UserResp{
			Id:             f.Id,
			Name:           f.Name,
			DisplayName:    f.DisplayName,
			Country:        f.Country,
			ProfilePicture: f.ProfilePicture,
			BannerImage:    f.BannerImage,
			Verified:       f.Verified,
			Theme:          f.Theme,
		})
	}

	return &followers, nil
}
