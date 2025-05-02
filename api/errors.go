package api

import (
	"fmt"
	"net/http"
)

type Error struct {
	err  string
	code int
}

var (
	NotFound            = Error{err: "resource not found", code: 404}
	InternalServerError = Error{err: "internal server error", code: 500}
	Forbidden           = Error{err: "forbidden", code: 403}
)

func SendError(w http.ResponseWriter, err Error) {
	http.Error(w, fmt.Sprintf("{\"error\": \"%s\"}", err.err), err.code)
}
