package api

import (
	"fmt"
	"net/http"
)

func JSONError(w http.ResponseWriter, code int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	fmt.Fprintf(w, "{\"error\": \"%s\"}", message)
}
