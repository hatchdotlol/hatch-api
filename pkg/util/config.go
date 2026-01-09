package util

import (
	"log/slog"
	"os"
	"strconv"
	"strings"
	"time"
)

func InitConfig() {
	_mods := strings.Split(os.Getenv("MODS"), ",")
	mods := make(map[string]bool)
	for _, m := range _mods {
		mods[m] = true
	}

	if len(mods) == 0 {
		slog.Warn("No mods configured")
	}

	var loggingWebhook string
	if w := os.Getenv("LOGGING_WEBHOOK"); w != "" {
		loggingWebhook = w
	}

	var reportWebhook string
	if w := os.Getenv("REPORT_WEBHOOK"); w != "" {
		reportWebhook = w
	}

	githubCfg := func() *github {
		id := os.Getenv("GITHUB_CLIENT_ID")
		secret := os.Getenv("GITHUB_CLIENT_SECRET")
		redirect := os.Getenv("GITHUB_REDIRECT_URI")
		if id == "" || secret == "" || redirect == "" {
			return nil
		}
		return &github{
			ClientID:     id,
			ClientSecret: secret,
			RedirectURI:  redirect,
		}
	}()

	port, _ := strconv.Atoi(os.Getenv("EMAIL_SMTP_PORT"))

	Config = config{
		StartTime:      time.Now().Unix(),
		AdminKey:       os.Getenv("ADMIN_KEY"),
		Mods:           mods,
		LoggingWebhook: &loggingWebhook,
		ReportWebhook:  &reportWebhook,
		PerPage:        50,
		Mail: &mail{
			PlatformName:      os.Getenv("EMAIL_PLATFORM_NAME"),
			PlatformLogo:      os.Getenv("EMAIL_PLATFORM_LOGO"),
			PlatformFrontend:  os.Getenv("EMAIL_PLATFORM_FRONTEND"),
			FromName:          os.Getenv("EMAIL_FROM_NAME"),
			FromAddress:       os.Getenv("EMAIL_FROM_ADDRESS"),
			EmailSMTPHost:     os.Getenv("EMAIL_SMTP_HOST"),
			EmailSMTPPort:     port,
			EmailSMTPUsername: os.Getenv("EMAIL_SMTP_USERNAME"),
			EmailSMTPPassword: os.Getenv("EMAIL_SMTP_PASSWORD"),
		},
		GitHub: githubCfg,
	}
}

var Config config

type config struct {
	StartTime      int64
	AdminKey       string
	Mods           map[string]bool
	LoggingWebhook *string
	ReportWebhook  *string
	PerPage        int
	Mail           *mail
	GitHub         *github
}

type mail struct {
	PlatformName      string
	PlatformLogo      string
	PlatformFrontend  string
	FromName          string
	FromAddress       string
	EmailSMTPHost     string
	EmailSMTPPort     int
	EmailSMTPUsername string
	EmailSMTPPassword string
}

type github struct {
	ClientID     string
	ClientSecret string
	RedirectURI  string
}
