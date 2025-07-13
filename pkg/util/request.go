package util

import (
	"io"
	"net/http"
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
