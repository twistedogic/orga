package bolt

import (
	"context"

	"github.com/google/uuid"

	"github.com/twistedogic/orga/pkg/backend"
)

func (b Backend) AddBoard(ctx context.Context, board *backend.Board) error {
	id := uuid.NewString()
	board.Id = id
	return b.BoardHandler.Set(id, board)
}

func (b Backend) GetBoard(ctx context.Context, id string) (*backend.Board, error) {
	board := new(backend.Board)
	if err := b.BoardHandler.Get(id, board); err != nil {
		return nil, err
	}
	board.SetBackend(b)
	return board, nil
}

func (b Backend) UpdateBoard(ctx context.Context, board *backend.Board) error {
	if _, err := b.GetBoard(ctx, board.Id); err != nil {
		return err
	}
	return b.BoardHandler.Set(board.Id, board)
}

func (b Backend) DeleteBoard(ctx context.Context, id string) error {
	return b.BoardHandler.Delete(id)
}

func (b Backend) ListBoards(ctx context.Context) ([]*backend.Board, error) {
	ids, err := b.BoardHandler.List()
	if err != nil {
		return nil, err
	}
	boards := make([]*backend.Board, len(ids))
	for i, id := range ids {
		board, err := b.GetBoard(ctx, id)
		if err != nil {
			return nil, err
		}
		boards[i] = board
	}
	return boards, nil
}
