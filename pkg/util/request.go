package util

import (
	"io"
	"net/http"
	"strconv"
)

func HttpBody(r *http.Request) []byte {
	body := r.Body
	defer body.Close()

	bodyb, err := io.ReadAll(body)
	if err != nil {
		return nil
	}

	return bodyb
}

// read the ?page query param
func Page(r *http.Request) int {
	var page int

	if _page := r.URL.Query().Get("page"); _page != "" {
		_page, err := strconv.Atoi(_page)
		if err != nil {
			return 0
		}
		page = _page
	} else {
		page = 0
	}

	return page
}
