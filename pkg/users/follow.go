package users

import (
	"slices"
	"strconv"
	"strings"

	"github.com/hatchdotlol/hatch-api/pkg/db"
)

func commaSplit(c rune) bool {
	return c == ','
}

func Follow(user string, follower int64, follow bool) error {
	followeeUser, err := UserByName(user, true)
	if err != nil {
		return err
	}

	// add follower to user's followers
	_followers := ""
	if followeeUser.Followers != nil {
		_followers = *followeeUser.Followers
	}

	strFollower := strconv.Itoa(int(follower))

	followers := strings.FieldsFunc(_followers, commaSplit)

	if follow {
		followers = append(followers, strFollower)
		slices.Sort(followers)
		followers = slices.Compact(followers)
	} else {
		removeUser := make([]string, 0, len(followers)-1)
		for _, f := range followers {
			if f != strFollower {
				removeUser = append(removeUser, f)
			}
		}
		followers = removeUser
	}

	tx, err := db.Db.Begin()
	if err != nil {
		return err
	}

	if _, err := tx.Exec(
		"UPDATE users SET followers = ? WHERE id = ?",
		strings.Join(followers, ","),
		followeeUser.Id,
	); err != nil {
		return err
	}

	// add user to follower's following
	followerUser, err := UserFromRow(db.Db.QueryRow("SELECT * FROM users WHERE id = ?", follower))
	if err != nil {
		return err
	}

	_following := ""
	if followeeUser.Following != nil {
		_following = *followeeUser.Following
	}

	following := strings.FieldsFunc(_following, commaSplit)

	if follow {
		following = append(following, strconv.Itoa(int(followerUser.Id)))
		slices.Sort(following)
		following = slices.Compact(following)
	} else {
		removeUser := make([]string, 0, len(following)-1)
		for _, f := range following {
			if f != strFollower {
				removeUser = append(removeUser, f)
			}
		}
		following = removeUser
	}

	if _, err := tx.Exec(
		"UPDATE users SET following = ? WHERE id = ?",
		strings.Join(following, ","),
		follower,
	); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return err
	}

	return nil
}
