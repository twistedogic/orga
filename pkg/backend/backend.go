package backend

import (
	"context"
	"time"
)

type Board struct {
	Id, Name string
}

type List struct {
	BoardId, Id, Name string
	Pos               float64
}

type Card struct {
	LastUpdate                    time.Time
	ListId, Id, Name, Description string
	Value, Effort, Work           int
	Labels                        []Label
}

type Label struct {
	Color, Name string
}

type BoardHandler interface {
	ListBoards(context.Context) ([]Board, error)
	GetBoard(context.Context, string) (Board, error)
	AddBoard(context.Context, Board) error
	UpdateBoard(context.Context, Board) error
	DeleteBoard(context.Context, string) error
}

type ListHandler interface {
	ListLists(context.Context, string) ([]List, error)
	GetList(context.Context, string) (List, error)
	AddList(context.Context, List) error
	UpdateList(context.Context, List) error
	DeleteList(context.Context, string) error
}

type CardHandler interface {
	ListCards(context.Context, string) ([]Card, error)
	GetCard(context.Context, string) (Card, error)
	AddCard(context.Context, Card) error
	UpdateCard(context.Context, Card) error
	DeleteCard(context.Context, string) error
}

type Backend interface {
	BoardHandler
	ListHandler
	CardHandler
}
