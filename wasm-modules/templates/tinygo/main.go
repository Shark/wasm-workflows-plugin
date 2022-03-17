package main

import (
	"fmt"
	"os"
)

func main() {
	fmt.Fprintf(
		os.Stdout,
		"{\"phase\":\"%s\",\"message\":\"%s\"}",
		"Succeeded",
		"Hello from Golang!",
	)
}
