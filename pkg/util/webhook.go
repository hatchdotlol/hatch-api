package util

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/getsentry/sentry-go"
)

var discordClient = http.Client{
	Timeout: time.Duration(10) * time.Second,
}

// Log message to discord and sentry
func LogMessage(content string) {
	fmt.Printf("Config: %v\n", Config)
	if Config.LoggingWebhook != nil && *Config.LoggingWebhook != "" {
		body, _ := json.Marshal(map[string]string{
			"content": content,
		})
		resp, err := discordClient.Post(*Config.LoggingWebhook, "application/json", bytes.NewBuffer(body))
		if err != nil {
			log.Fatal(err)
			sentry.CaptureException(err)
		}

		if resp.StatusCode != http.StatusOK {
			sentry.CaptureException(errors.New("failed to log to discord"))
		}
	}

	sentry.CaptureMessage(content)
}
