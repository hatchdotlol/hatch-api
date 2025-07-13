package api

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"net/mail"
	"regexp"
	"time"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"golang.org/x/crypto/bcrypt"
)

func AuthRouter() *chi.Mux {
	r := chi.NewRouter()
	r.Post("/register", register)
	r.Post("/login", login)

	r.Group(func(r chi.Router) {
		r.Use(EnsureUser)
		r.Get("/me", me)
		r.Get("/logout", logout)
		r.Get("/delete", delete)
	})

	return r
}

var validUsername = regexp.MustCompile(`^[\w-]+$`)

func register(w http.ResponseWriter, r *http.Request) {
	adminKey := r.Header.Get("Admin-Key")
	if util.Config.AdminKey != "" && adminKey != util.Config.AdminKey {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var form models.RegisterForm

	body := util.HttpBody(r)
	if body == nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	if err := json.Unmarshal(body, &form); err != nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}

	// form validation
	if !validUsername.MatchString(form.Username) || len(form.Username) > 20 {
		http.Error(w, "Invalid username", http.StatusBadRequest)
		return
	}

	if _, err := mail.ParseAddress(form.Email); err != nil {
		http.Error(w, "Invalid email", http.StatusBadRequest)
		return
	}

	if util.Entropy(form.Password) < 28 {
		http.Error(w, "Password is too weak", http.StatusBadRequest)
		return
	}

	user, err := users.UserByName(form.Username, true)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}
	if user != nil {
		http.Error(w, "That username already exists", http.StatusBadRequest)
		return
	}

	hash, err := bcrypt.GenerateFromPassword([]byte(form.Password), 10)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	theme := "#ff5d59"
	u := users.UserRow{
		Name:                form.Username,
		Pw:                  string(hash),
		DisplayName:         &form.Username,
		Country:             "Location Not Given",
		Bio:                 nil,
		HighlightedProjects: nil,
		ProfilePicture:      "default",
		JoinDate:            time.Now().String(),
		BannerImage:         nil,
		Followers:           nil,
		Following:           nil,
		Verified:            false,
		Email:               form.Email,
		Banned:              false,
		Theme:               &theme,
	}

	if err := u.Insert(); err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to register", http.StatusInternalServerError)
		return
	}

	// TODO: email verif

	fmt.Fprint(w, "Welcome")
}

func login(w http.ResponseWriter, r *http.Request) {
	var form models.LoginForm

	body := util.HttpBody(r)
	if body == nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}
	if err := json.Unmarshal(body, &form); err != nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}

	u, err := users.UserByName(form.Username, true)
	if err != nil {
		http.Error(w, "Invalid username/password", http.StatusBadRequest)
		return
	}

	if err := bcrypt.CompareHashAndPassword(
		[]byte(u.Pw),
		[]byte(form.Password),
	); err != nil {
		if err != bcrypt.ErrMismatchedHashAndPassword {
			sentry.CaptureException(err)
		}
		http.Error(w, "Invalid username/password", http.StatusBadRequest)
		return
	}

	token, err := users.GetOrCreateToken(u.Id)
	if err != nil {
		log.Print(err)
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	fmt.Fprintf(w, `{"token": "%s"}`, *token)
}

func logout(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(*users.UserRow)

	if err := users.RemoveTokens(user.Id); err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to log out", http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, "Logged out")
}

func me(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(*users.UserRow)

	resp, _ := json.Marshal(models.UserResp{
		Id:             user.Id,
		Name:           user.Name,
		DisplayName:    user.DisplayName,
		Country:        user.Country,
		Bio:            user.Bio,
		ProfilePicture: user.ProfilePicture,
		JoinDate:       user.JoinDate,
		BannerImage:    user.BannerImage,
		Verified:       user.Verified,
		Theme:          user.Theme,
		HatchTeam:      util.Config.Mods[user.Name],
		Banned:         &user.Banned,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

func delete(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(*users.UserRow)

	// check provided password
	body := util.HttpBody(r)
	if body != nil {
		http.Error(w, "Invalid form", http.StatusBadRequest)
		return
	}

	password := string(body)
	if err := bcrypt.CompareHashAndPassword(
		[]byte(user.Pw),
		[]byte(password),
	); err != nil {
		if err != bcrypt.ErrMismatchedHashAndPassword {
			sentry.CaptureException(err)
		}
		http.Error(w, "Invalid username/password", http.StatusBadRequest)
		return
	}

	go func() {
		if err := users.DeleteUser(user.Id); err != nil {
			sentry.CaptureException(err)
		}
	}()

	fmt.Fprint(w, "Account deletion scheduled. Please wait")
}
