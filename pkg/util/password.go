package util

import (
	"crypto/rand"
	"encoding/base64"
	"math"
	"slices"
	"strings"
)

func Entropy(password string) float64 {
	ch := charset(password)
	length := float64(len(password))

	return length * math.Log2(ch)
}

func charset(password string) float64 {
	bytes := []rune(password)

	var charset uint32 = 0

	if slices.ContainsFunc(bytes, func(ch rune) bool {
		return ch >= '0' && ch <= '9'
	}) {
		charset += 10
	}

	if slices.ContainsFunc(bytes, func(ch rune) bool {
		return ch >= 'a' && ch <= 'z'
	}) {
		charset += 26
	}

	if slices.ContainsFunc(bytes, func(ch rune) bool {
		return ch >= 'A' && ch <= 'Z'
	}) {
		charset += 26
	}

	if slices.ContainsFunc(bytes, func(ch rune) bool {
		return ch < '0' || (ch > '9' && ch < 'A') || (ch > 'Z' && ch < 'a') || ch > 'z'
	}) {
		charset += 33
	}

	return float64(charset)
}

func GenerateId(n int) (string, error) {
	b := make([]byte, n)
	_, err := rand.Read(b)
	if err != nil {
		return "", err
	}

	id := base64.URLEncoding.EncodeToString(b)
	id = strings.ReplaceAll(id, "-", "a")
	id = strings.ReplaceAll(id, "_", "b")
	id = strings.ReplaceAll(id, "=", "c")

	return id, err
}
