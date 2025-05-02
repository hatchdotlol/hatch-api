package api

func IPBanned(ip string) (bool, error) {
	rows, err := db.Query("SELECT address FROM ip_bans WHERE address = ?1", ip)
	if err != nil {
		return false, err
	}
	defer rows.Close()
	rows.Next()

	var userIp *string
	if err := rows.Scan(&userIp); err != nil {
		return false, err
	}

	return userIp != nil, nil
}
