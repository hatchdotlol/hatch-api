package api

import (
	"log/slog"
	"os"
	"strings"
	"time"
)

var (
	startTime      = time.Now().Unix()
	version        = os.Getenv("VERSION")
	adminKey       = os.Getenv("ADMIN_KEY")
	baseUrl        = os.Getenv("BASE_URL")
	minioAccessKey = os.Getenv("MINIO_ACCESS_KEY")
	minioSecretKey = os.Getenv("MINIO_SECRET_KEY")
	postalUrl      = os.Getenv("POSTAL_URL")
	postalKey      = os.Getenv("POSTAL_KEY")
	resendKey      = os.Getenv("RESEND_KEY")
	dbPath         = os.Getenv("DB_PATH")
	mods           = strings.Split(os.Getenv("MODS"), ",")
	loggingWebhook = os.Getenv("LOGGING_WEBHOOK")
	reportWebhook  = os.Getenv("REPORT_WEBHOOK")
)

type Config struct {
	startTime      int64
	version        string
	adminKey       string
	baseUrl        string
	minioAccessKey string
	minioSecretKey string
	postalUrl      string
	postalKey      string
	resendKey      *string
	dbPath         string
	mods           []string
	loggingWebhook *string
	reportWebhook  *string
}

var missingFields = version == "" || adminKey == "" || baseUrl == "" || minioAccessKey == "" || minioSecretKey == "" || postalUrl == "" || postalKey == "" || dbPath == ""

var config = GetConfig()

func GetConfig() Config {
	// if missingFields {
	// 	log.Fatalln("Missing env vars")
	// }

	if len(mods) == 0 {
		slog.Warn("No mods configured")
	}

	return Config{
		startTime,
		version,
		adminKey,
		baseUrl,
		minioAccessKey,
		minioSecretKey,
		postalUrl,
		postalKey,
		&resendKey,
		dbPath,
		mods,
		&loggingWebhook,
		&reportWebhook,
	}
}
