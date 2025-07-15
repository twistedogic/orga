package run

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/urfave/cli/v2"

	"github.com/twistedogic/orga/pkg/backend"
	"github.com/twistedogic/orga/pkg/backend/bolt"
	"github.com/twistedogic/orga/pkg/view"
)

var (
	boardVar string
	dbVar    string
	runFlags = []cli.Flag{
		&cli.StringFlag{
			Name:        "board",
			Aliases:     []string{"b"},
			Usage:       "board name to display",
			Destination: &boardVar,
			Value:       "Main Board",
		},
		&cli.StringFlag{
			Name:        "db",
			Aliases:     []string{"d"},
			Usage:       "database file path",
			Destination: &dbVar,
			Value:       "orga.db",
		},
	}
)

func Run(ctx *cli.Context) error {
	// Initialize backend
	backendInstance, err := bolt.New(dbVar)
	if err != nil {
		return fmt.Errorf("failed to initialize database: %w", err)
	}

	// Get or create board
	boards, err := backendInstance.ListBoards(context.Background())
	if err != nil {
		return fmt.Errorf("failed to list boards: %w", err)
	}

	var board *backend.Board
	for _, b := range boards {
		if b.Name == boardVar {
			board = b
			break
		}
	}

	if board == nil {
		// Create new board
		board = &backend.Board{
			Id:   generateID(),
			Name: boardVar,
		}
		board.SetBackend(backendInstance)
		if err := backendInstance.AddBoard(context.Background(), board); err != nil {
			return fmt.Errorf("failed to create board: %w", err)
		}
	} else {
		board.SetBackend(backendInstance)
	}

	// Initialize and run TUI
	model, err := view.New(context.Background(), board)
	if err != nil {
		return fmt.Errorf("failed to initialize view: %w", err)
	}

	return model.Run()
}

func generateID() string {
	return uuid.New().String()
}

func Command() *cli.Command {
	return &cli.Command{
		Name:   "run",
		Usage:  "run the TUI kanban board",
		Flags:  runFlags,
		Action: Run,
	}
}