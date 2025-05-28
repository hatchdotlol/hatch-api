package uploads

import (
	"fmt"
	"net/http"
	"time"
)

var client = http.Client{
	Timeout: time.Duration(10) * time.Second,
}

var assetCache = make(map[string]bool, 0)

// check if an asset in a project exists on scratch
func AssetExists(file string) (bool, error) {
	_, ok := assetCache[file]
	if ok {
		return true, nil
	}
	resp, err := client.Head(fmt.Sprintf("https://assets.scratch.mit.edu/internalapi/%s/get/", file))
	if err != nil {
		return false, err
	}
	exists := resp.StatusCode == http.StatusOK
	if exists {
		// i love heuristics^tm
		assetCache[file] = true
	}
	return exists, nil
}
