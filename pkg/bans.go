package api

import "github.com/hatchdotlol/hatch-api/pkg/db"

func IPBanned(ip string) (bool, error) {
	row := db.Db.QueryRow("SELECT address FROM ip_bans WHERE address = ?1", ip)
	if row != nil {
		return false, nil
	}

	var userIp *string
	if err := row.Scan(&userIp); err != nil {
		return false, err
	}

	return userIp != nil, nil
}
