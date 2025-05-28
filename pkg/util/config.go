package util

import (
	"log/slog"
	"os"
	"strings"
	"time"

	"github.com/joho/godotenv"
)

func InitConfig() {
	godotenv.Load()

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

	Config = config{
		StartTime:      time.Now().Unix(),
		Version:        os.Getenv("VERSION"),
		AdminKey:       os.Getenv("ADMIN_KEY"),
		ResendKey:      os.Getenv("RESEND_KEY"),
		Mods:           mods,
		LoggingWebhook: &loggingWebhook,
		ReportWebhook:  &reportWebhook,
		PerPage:        50,
	}
}

var Config config

type config struct {
	StartTime      int64
	Version        string
	AdminKey       string
	ResendKey      string
	Mods           map[string]bool
	LoggingWebhook *string
	ReportWebhook  *string
	PerPage        int
}
