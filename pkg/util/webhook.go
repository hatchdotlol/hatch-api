package util

import (
	"bytes"
	"errors"
	"fmt"
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
		body := fmt.Sprintf(`{"content": "%s"}`, content)
		resp, err := discordClient.Post(*Config.LoggingWebhook, "application/json", bytes.NewBuffer([]byte(body)))
		if err != nil {
			sentry.CaptureException(err)
		}

		if resp.StatusCode != http.StatusOK {
			sentry.CaptureException(errors.New("failed to log to discord"))
		}
	}

	sentry.CaptureMessage(content)
}
