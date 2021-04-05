package cmd

import (
	"github.com/urfave/cli/v2"

	"github.com/twistedogic/orga/cmd/configure"
)

func App() *cli.App {
	return &cli.App{
		Name:  "orga",
		Usage: "Agile Trello for one",
		Commands: []*cli.Command{
			configure.Command(),
		},
	}
}
