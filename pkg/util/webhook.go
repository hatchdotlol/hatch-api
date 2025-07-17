package util

import (
	"bytes"
	"encoding/json"
	"errors"
	"net/http"
	"time"

	"github.com/getsentry/sentry-go"
)

var discordClient = http.Client{
	Timeout: time.Duration(10) * time.Second,
}

// Log message to discord and sentry
func LogMessage(content string) {
	if Config.LoggingWebhook != nil {
		body, _ := json.Marshal(map[string]string{
			"content": content,
		})
		resp, err := discordClient.Post(*Config.LoggingWebhook, "application/json", bytes.NewBuffer(body))
		if err != nil {
			sentry.CaptureException(err)
		}

		if resp.StatusCode != http.StatusOK {
			sentry.CaptureException(errors.New("failed to log to discord"))
		}
	}

	sentry.CaptureMessage(content)
}
