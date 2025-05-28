package api

import (
	"context"
	"fmt"
	"net/http"

	"github.com/hatchdotlol/hatch-api/pkg/users"
)

func goodUser(w http.ResponseWriter, r *http.Request) *users.UserRow {
	token := r.Header.Get("Token")

	if token == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return nil
	}

	u, err := users.UserByToken(token)
	if err != nil {
		fmt.Print(err)
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return nil
	}

	if u.Banned || !u.Verified {
		http.Error(w, "Forbidden", http.StatusForbidden)
		return nil
	}

	return u
}

type userKey struct{}

var User = userKey{}

func EnsureVerified(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := goodUser(w, r)
		if u == nil {
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}
