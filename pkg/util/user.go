package util

import (
	"strconv"
	"strings"
)

func ParseIds(csv *string) []int64 {
	if csv == nil {
		return []int64{}
	}
	split := strings.Split(*csv, ",")
	ids := make([]int64, 0, len(split))
	for _, s := range split {
		num, _ := strconv.Atoi(s)
		ids = append(ids, int64(num))
	}
	return ids
}
