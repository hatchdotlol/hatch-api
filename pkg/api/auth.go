package api

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/mail"
	"net/url"
	"regexp"
	"strconv"
	"strings"
	"time"

	"github.com/getsentry/sentry-go"
	"github.com/go-chi/chi/v5"
	"github.com/hatchdotlol/hatch-api/pkg/emails"
	"github.com/hatchdotlol/hatch-api/pkg/models"
	"github.com/hatchdotlol/hatch-api/pkg/users"
	"github.com/hatchdotlol/hatch-api/pkg/util"
	"golang.org/x/crypto/bcrypt"
)

func AuthRouter() *chi.Mux {
	r := chi.NewRouter()
	r.Post("/register", register)
	r.Post("/login", login)
	r.Get("/github/login", githubLogin)
	r.Get("/github/callback", githubCallback)

	r.Group(func(r chi.Router) {
		r.Use(EnsureUser)
		r.Get("/me", me)
		r.Get("/logout", logout)
		r.Get("/delete", deleteAccount)
		r.Post("/reverify", reverify)
	})

	return r
}

var validUsername = regexp.MustCompile(`^[\w-]+$`)

const registerMessage = "*[%s](https://dev.hatch.lol/user?u=%s) has registered.* 👤"

const githubStateCookie = "github_state"

type githubAccessTokenResponse struct {
	AccessToken string `json:"access_token"`
	Scope       string `json:"scope"`
	TokenType   string `json:"token_type"`
}

type githubUserResponse struct {
	ID        int64  `json:"id"`
	Login     string `json:"login"`
	Name      string `json:"name"`
	AvatarURL string `json:"avatar_url"`
	Email     string `json:"email"`
}

func register(w http.ResponseWriter, r *http.Request) {
	adminKey := r.Header.Get("Admin-Key")
	if util.Config.AdminKey != "" && adminKey != util.Config.AdminKey {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	var form models.Register

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
	if err != nil && err != sql.ErrNoRows {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}
	if user != (users.User{}) {
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
	u := users.User{
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

	go func() {
		util.LogMessage(fmt.Sprintf(registerMessage, form.Username, form.Username))
		if err := emails.SendVerificationEmail(form.Username, form.Email); err != nil {
			util.LogMessage(fmt.Sprintf("We could not send a verification email to %s.", form.Username))
			sentry.CaptureException(err)
		}
	}()

	fmt.Fprint(w, "Welcome")
}

func githubLogin(w http.ResponseWriter, r *http.Request) {
	if util.Config.GitHub == nil {
		http.Error(w, "GitHub login not configured", http.StatusServiceUnavailable)
		return
	}

	state, err := util.GenerateId(16)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	http.SetCookie(w, &http.Cookie{
		Name:     githubStateCookie,
		Value:    state,
		Path:     "/",
		HttpOnly: true,
		SameSite: http.SameSiteLaxMode,
		Secure:   r.TLS != nil,
		Expires:  time.Now().Add(10 * time.Minute),
	})

	params := url.Values{}
	params.Set("client_id", util.Config.GitHub.ClientID)
	params.Set("redirect_uri", util.Config.GitHub.RedirectURI)
	params.Set("scope", "read:user")
	params.Set("state", state)

	http.Redirect(w, r, "https://github.com/login/oauth/authorize?"+params.Encode(), http.StatusFound)
}

func githubCallback(w http.ResponseWriter, r *http.Request) {
	if util.Config.GitHub == nil {
		http.Error(w, "GitHub login not configured", http.StatusServiceUnavailable)
		return
	}

	code := r.URL.Query().Get("code")
	state := r.URL.Query().Get("state")
	if code == "" || state == "" {
		http.Error(w, "Missing code/state", http.StatusBadRequest)
		return
	}

	stateCookie, err := r.Cookie(githubStateCookie)
	if err != nil || stateCookie.Value != state {
		http.Error(w, "Invalid state", http.StatusUnauthorized)
		return
	}

	token, err := exchangeGithubCode(code)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Could not login with GitHub", http.StatusBadGateway)
		return
	}

	profile, err := fetchGithubProfile(token)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Could not login with GitHub", http.StatusBadGateway)
		return
	}

	email := profile.Email
	if email == "" {
		email = fmt.Sprintf("%s@users.noreply.github.com", profile.Login)
	}

	githubID := strconv.FormatInt(profile.ID, 10)
	existing, err := users.UserByGithubID(githubID)
	if err != nil && !errors.Is(err, sql.ErrNoRows) {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	if existing != (users.User{}) {
		tkn, err := users.GetOrCreateToken(existing.Id)
		if err != nil {
			sentry.CaptureException(err)
			w.WriteHeader(http.StatusInternalServerError)
			return
		}

		dest := r.URL.Query().Get("redirect")
		if dest == "" {
			dest = "https://hatch.lol"
		}
		http.Redirect(w, r, dest+"#token="+url.QueryEscape(tkn), http.StatusFound)
		return
	}

	username := profile.Login
	if username == "" || !validUsername.MatchString(username) || len(username) > 20 {
		username = fmt.Sprintf("github-%s", githubID)
	}

	if user, err := users.UserByName(username, true); err == nil && user != (users.User{}) {
		if user.GithubID == nil {
			if err := users.AttachGithub(user.Id, githubID); err != nil {
				sentry.CaptureException(err)
				http.Error(w, "Failed to link GitHub", http.StatusInternalServerError)
				return
			}

			tkn, err := users.GetOrCreateToken(user.Id)
			if err != nil {
				sentry.CaptureException(err)
				w.WriteHeader(http.StatusInternalServerError)
				return
			}

			dest := r.URL.Query().Get("redirect")
			if dest == "" {
				dest = "https://hatch.lol"
			}
			http.Redirect(w, r, dest+"#token="+url.QueryEscape(tkn), http.StatusFound)
			return
		}

		suffix, genErr := util.GenerateId(6)
		if genErr != nil {
			sentry.CaptureException(genErr)
			http.Error(w, "Something went wrong", http.StatusInternalServerError)
			return
		}
		if len(suffix) > 6 {
			suffix = suffix[:6]
		}
		username = fmt.Sprintf("%s-%s", username, suffix)
	}

	password, err := util.GenerateId(32)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	hash, err := bcrypt.GenerateFromPassword([]byte(password), 10)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Something went wrong", http.StatusInternalServerError)
		return
	}

	var displayName *string
	if profile.Name != "" {
		displayName = &profile.Name
	}

	theme := "#ff5d59"
	newUser := users.User{
		Name:                username,
		Pw:                  string(hash),
		DisplayName:         displayName,
		Country:             "Location Not Given",
		Bio:                 nil,
		HighlightedProjects: nil,
		ProfilePicture:      "default",
		JoinDate:            time.Now().String(),
		BannerImage:         nil,
		Followers:           nil,
		Following:           nil,
		Verified:            true,
		Email:               email,
		Banned:              false,
		Theme:               &theme,
		GithubID:            &githubID,
	}

	if err := newUser.Insert(); err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to create account", http.StatusInternalServerError)
		return
	}

	createdUser, err := users.UserByGithubID(githubID)
	if err != nil {
		sentry.CaptureException(err)
		http.Error(w, "Failed to create account", http.StatusInternalServerError)
		return
	}

	tkn, err := users.GetOrCreateToken(createdUser.Id)
	if err != nil {
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	dest := r.URL.Query().Get("redirect")
	if dest == "" {
		dest = "https://hatch.lol"
	}
	http.Redirect(w, r, dest+"#token="+url.QueryEscape(tkn), http.StatusFound)
}

func login(w http.ResponseWriter, r *http.Request) {
	var form models.Login

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
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	fmt.Fprintf(w, `{"token": "%s"}`, token)
}

func reverify(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(users.User)

	sent, err := emails.VerificationEmailSent(user.Name)
	if err != nil {
		fmt.Printf("err: %v\n", err)
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	if sent {
		http.Error(w, "Verification email already sent", http.StatusBadRequest)
		return
	}

	go func() {
		if err := emails.SendVerificationEmail(user.Name, user.Email); err != nil {
			util.LogMessage(fmt.Sprintf("We could not send a verification email to %s.", user.Name))
			sentry.CaptureException(err)
		}
	}()

	fmt.Fprint(w, "Verification email resent")
}

func logout(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(users.User)

	if err := users.RemoveTokens(user.Id); err != nil {
		sentry.CaptureException(err)
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, "Logged out")
}

func me(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(users.User)

	resp, _ := json.Marshal(users.UserJSON{
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
		Checkmark:      user.Checkmark,
	})

	w.Header().Add("Content-Type", "application/json")
	fmt.Fprint(w, string(resp))
}

func deleteAccount(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value(User).(users.User)

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

func exchangeGithubCode(code string) (string, error) {
	data := url.Values{}
	data.Set("client_id", util.Config.GitHub.ClientID)
	data.Set("client_secret", util.Config.GitHub.ClientSecret)
	data.Set("code", code)
	data.Set("redirect_uri", util.Config.GitHub.RedirectURI)

	req, err := http.NewRequest(http.MethodPost, "https://github.com/login/oauth/access_token", strings.NewReader(data.Encode()))
	if err != nil {
		return "", err
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return "", fmt.Errorf("github token exchange failed: %s", string(body))
	}

	var token githubAccessTokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&token); err != nil {
		return "", err
	}

	if token.AccessToken == "" {
		return "", errors.New("missing GitHub access token")
	}

	return token.AccessToken, nil
}

func fetchGithubProfile(accessToken string) (githubUserResponse, error) {
	req, err := http.NewRequest(http.MethodGet, "https://api.github.com/user", nil)
	if err != nil {
		return githubUserResponse{}, err
	}
	req.Header.Set("Authorization", "Bearer "+accessToken)
	req.Header.Set("Accept", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return githubUserResponse{}, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return githubUserResponse{}, fmt.Errorf("github user fetch failed: %s", string(body))
	}

	var user githubUserResponse
	if err := json.NewDecoder(resp.Body).Decode(&user); err != nil {
		return githubUserResponse{}, err
	}

	return user, nil
}
