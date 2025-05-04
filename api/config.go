package api

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

	config = Config{
		startTime:      time.Now().Unix(),
		version:        os.Getenv("VERSION"),
		adminKey:       os.Getenv("ADMIN_KEY"),
		baseUrl:        os.Getenv("BASE_URL"),
		resendKey:      os.Getenv("RESEND_KEY"),
		ingestDir:      os.Getenv("INGEST_DIR"),
		mods:           mods,
		loggingWebhook: &loggingWebhook,
		reportWebhook:  &reportWebhook,
		perPage:        50,
	}
}

var config Config

type Config struct {
	startTime      int64
	version        string
	adminKey       string
	baseUrl        string
	resendKey      string
	ingestDir      string
	mods           map[string]bool
	loggingWebhook *string
	reportWebhook  *string
	perPage        int
}
