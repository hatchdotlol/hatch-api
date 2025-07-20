package api

import (
	"context"
	"net/http"

	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

// Check if a user exists from a token
func GoodUser(w http.ResponseWriter, r *http.Request) *users.User {
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

// Ensure a correct token exists
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

// Ensure the user from given token is verified
func EnsureVerified(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r)
		if u == nil {
			return
		}

		if u.Banned {
			http.Error(w, "You are banned. If you think this was a mistake, please e-mail contact[at]hatch.lol to appeal", http.StatusForbidden)
			return
		}

		if !u.Verified {
			http.Error(w, "Your email is not verified", http.StatusUnauthorized)
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// Ensure the user is a mod
func EnsureMod(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r)
		if u == nil {
			return
		}

		if !util.Config.Mods[u.Name] {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}
