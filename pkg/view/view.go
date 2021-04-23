package view

import (
	"context"

	"github.com/rivo/tview"

	"github.com/twistedogic/orga/pkg/backend"
	"github.com/twistedogic/orga/pkg/config"
)

type View struct {
	context.Context
	*tview.Application
	*backend.Board
}

func New(ctx context.Context, board *backend.Board) (View, error) {
	v := View{
		Context:     ctx,
		Application: tview.NewApplication(),
		Board:       board,
	}
	err := v.Init(ctx)
	return v, err
}

func (v View) bootstrap(ctx context.Context) error {
	lists, err := v.Lists(ctx)
	if err != nil {
		return err
	}
	if len(lists) != 0 {
		return nil
	}
	for i, l := range config.DefaultList {
		lists = append(lists, &backend.List{
			Name: l,
			Pos:  float64(i),
		})
	}
	return v.AddLists(ctx, lists...)
}

func (v View) Init(ctx context.Context) error {
	if err := v.bootstrap(ctx); err != nil {
		return err
	}
	return nil
}
