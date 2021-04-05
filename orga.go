package main

import (
	"log"
	"os"

	"github.com/twistedogic/orga/cmd"
)

func main() {
	if err := cmd.App().Run(os.Args); err != nil {
		log.Fatal(err)
	}
}
