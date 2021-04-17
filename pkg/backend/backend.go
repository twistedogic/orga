package backend

import (
	"context"
	"sort"
	"time"
)

type Board struct {
	Backend
	Id, Name string
}

func (b *Board) List(ctx context.Context) ([]*List, error) {
	lists, err := b.ListLists(ctx, b.Id)
	if err != nil {
		return nil, err
	}
	sort.Slice(lists, func(i, j int) bool {
		return lists[i].Pos < lists[j].Pos
	})
	return lists, nil
}

func (b *Board) Update(ctx context.Context) error {
	return b.UpdateBoard(ctx, b)
}

type List struct {
	Backend
	BoardId, Id, Name string
	Pos               float64
}

func (l *List) Board(ctx context.Context) (*Board, error) {
	return l.GetBoard(ctx, l.BoardId)
}

func (l *List) List(ctx context.Context) ([]*Card, error) {
	cards, err := l.ListCards(ctx, l.Id)
	if err != nil {
		return nil, err
	}
	sort.Slice(cards, func(i, j int) bool {
		return cards[i].HasHigherPriority(cards[j])
	})
	return cards, nil
}

func (l *List) Sort(ctx context.Context) error {
	cards, err := l.List(ctx)
	if err != nil {
		return err
	}
	for i, c := range cards {
		c.Pos = float64(i)
		if err := c.Update(ctx); err != nil {
			return err
		}
	}
	return nil
}

func (l *List) Update(ctx context.Context) error {
	return l.UpdateList(ctx, l)
}

type Card struct {
	Backend
	LastUpdate                    time.Time
	ListId, Id, Name, Description string
	Value, Effort, Work           int
	Labels                        []Label
	Pos                           float64
}

func (c *Card) List(ctx context.Context) (*List, error) {
	return c.GetList(ctx, c.ListId)
}

func (c *Card) Update(ctx context.Context) error {
	return c.UpdateCard(ctx, c)
}

func (c *Card) HasHigherPriority(o *Card) bool {
	switch {
	case c.Value == o.Value:
		return c.Effort < o.Effort
	default:
		return c.Value > o.Value
	}
}

type Label struct {
	Color, Name string
}

type BoardHandler interface {
	ListBoards(context.Context) ([]*Board, error)
	GetBoard(context.Context, string) (*Board, error)
	AddBoard(context.Context, *Board) error
	UpdateBoard(context.Context, *Board) error
	DeleteBoard(context.Context, string) error
}

type ListHandler interface {
	ListLists(context.Context, string) ([]*List, error)
	GetList(context.Context, string) (*List, error)
	AddList(context.Context, *List) error
	UpdateList(context.Context, *List) error
	DeleteList(context.Context, string) error
}

type CardHandler interface {
	ListCards(context.Context, string) ([]*Card, error)
	GetCard(context.Context, string) (*Card, error)
	AddCard(context.Context, *Card) error
	UpdateCard(context.Context, *Card) error
	DeleteCard(context.Context, string) error
}

type Backend interface {
	BoardHandler
	ListHandler
	CardHandler
}
