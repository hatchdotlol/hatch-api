package api

import (
	"context"
	"net/http"

	"github.com/hatchdotlol/hatch-api/pkg/users"
)

func GoodUser(w http.ResponseWriter, r *http.Request) *users.UserRow {
	token := r.Header.Get("Token")

	if token == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return nil
	}

	u, err := users.UserByToken(token)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return nil
	}

	return u
}

type userKey struct{}

var User = userKey{}

func EnsureUser(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r)
		if u == nil {
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

func EnsureVerified(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r)
		if u == nil {
			return
		}

		if u.Banned || !u.Verified {
			http.Error(w, "Forbidden", http.StatusForbidden)
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}
