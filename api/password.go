package api

import (
	"math"
	"slices"
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
