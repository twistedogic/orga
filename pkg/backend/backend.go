package backend

import (
	"context"
	"sort"
	"time"
)

type Board struct {
	backend  Backend `json:"-"`
	Id, Name string
}

func (b *Board) SetBackend(be Backend) {
	b.backend = be
}

func (b *Board) Delete(ctx context.Context) error {
	return b.backend.DeleteBoard(ctx, b.Id)
}

func (b *Board) Lists(ctx context.Context) ([]*List, error) {
	lists, err := b.backend.ListLists(ctx, b.Id)
	if err != nil {
		return nil, err
	}
	sort.Slice(lists, func(i, j int) bool {
		return lists[i].Pos < lists[j].Pos
	})
	return lists, nil
}

func (b *Board) Update(ctx context.Context) error {
	return b.backend.UpdateBoard(ctx, b)
}

func (b *Board) AddLists(ctx context.Context, lists ...*List) error {
	for _, list := range lists {
		list.BoardId = b.Id
		if err := b.backend.AddList(ctx, list); err != nil {
			return err
		}
	}
	return nil
}

type List struct {
	backend           Backend `json:"-"`
	BoardId, Id, Name string
	Pos               float64
}

func (l *List) SetBackend(be Backend) {
	l.backend = be
}

func (l *List) Board(ctx context.Context) (*Board, error) {
	return l.backend.GetBoard(ctx, l.BoardId)
}

func (l *List) Cards(ctx context.Context) ([]*Card, error) {
	cards, err := l.backend.ListCards(ctx, l.Id)
	if err != nil {
		return nil, err
	}
	sort.Slice(cards, func(i, j int) bool {
		return cards[i].HasHigherPriority(cards[j])
	})
	return cards, nil
}

func (l *List) Sort(ctx context.Context) error {
	cards, err := l.Cards(ctx)
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
	return l.backend.UpdateList(ctx, l)
}

func (l *List) Delete(ctx context.Context) error {
	return l.backend.DeleteList(ctx, l.Id)
}

func (l *List) AddCards(ctx context.Context, cards ...*Card) error {
	for _, card := range cards {
		card.ListId = l.Id
		if err := l.backend.AddCard(ctx, card); err != nil {
			return err
		}
	}
	return nil
}

type Card struct {
	backend                       Backend `json:"-"`
	LastUpdate                    time.Time
	ListId, Id, Name, Description string
	Value, Effort, Work           int
	Labels                        []Label
	Pos                           float64
}

func (c *Card) SetBackend(be Backend) {
	c.backend = be
}

func (c *Card) List(ctx context.Context) (*List, error) {
	return c.backend.GetList(ctx, c.ListId)
}

func (c *Card) Update(ctx context.Context) error {
	return c.backend.UpdateCard(ctx, c)
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
