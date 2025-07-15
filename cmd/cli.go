package cmd

import (
	"github.com/urfave/cli/v2"

	"github.com/twistedogic/orga/cmd/run"
)

func App() *cli.App {
	return &cli.App{
		Name:  "orga",
		Usage: "Local Kanban board for agile task management",
		Commands: []*cli.Command{
			run.Command(),
		},
	}
}
