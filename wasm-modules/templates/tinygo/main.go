package main

import (
	"os"
)

func main() {
	out := []byte("{\"phase\":\"Succeeded\",\"message\":\"Hello\",\"outputs\":{\"artifacts\":[],\"parameters\":[]}}")
	err := os.WriteFile("/work/result.json", out, 0644)
	if err != nil {
		panic(err)
	}
}
