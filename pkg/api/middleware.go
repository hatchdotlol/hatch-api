package api

import (
	"context"
	"net/http"

	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
)

// Check if a user exists from a token
func GoodUser(w http.ResponseWriter, r *http.Request, ws bool) users.User {
	var token string
	if ws {
		token = r.URL.Query().Get("token")
	} else {
		token = r.Header.Get("Token")
	}

	if token == "" {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return users.User{}
	}

	u, err := users.UserByToken(token)
	if err != nil {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return users.User{}
	}

	return u
}

type userKey struct{}

var User = userKey{}

// Ensure a correct token exists
func EnsureUser(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r, false)
		if u == (users.User{}) {
			return
		}

		ctx := context.WithValue(r.Context(), User, u)
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// Ensure the user from given token is verified
func EnsureVerified(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r, false)
		if u == (users.User{}) {
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

// god this sucks

// Ensure the user from given token is verified over a WebSocket
func EnsureVerifiedWs(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		u := GoodUser(w, r, true)
		if u == (users.User{}) {
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
		u := GoodUser(w, r, false)
		if u == (users.User{}) {
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
