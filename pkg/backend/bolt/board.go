package bolt

import (
	"context"

	"github.com/google/uuid"
	bolt "go.etcd.io/bbolt"

	"github.com/twistedogic/orga/pkg/backend"
)

type BoardHandler struct {
	handler Store
}

func NewBoardHandler(db *bolt.DB) (BoardHandler, error) {
	h := NewStore(boardBucketName, db)
	return BoardHandler{
		handler: h,
	}, h.Init()
}

func (b BoardHandler) AddBoard(ctx context.Context, board *backend.Board) error {
	id := uuid.NewString()
	board.Id = id
	return b.handler.Set(id, &board)
}

func (b BoardHandler) GetBoard(ctx context.Context, id string) (*backend.Board, error) {
	board := new(backend.Board)
	err := b.handler.Get(id, board)
	return board, err
}

func (b BoardHandler) UpdateBoard(ctx context.Context, board *backend.Board) error {
	if _, err := b.GetBoard(ctx, board.Id); err != nil {
		return err
	}
	return b.handler.Set(board.Id, board)
}

func (b BoardHandler) DeleteBoard(ctx context.Context, id string) error {
	return b.handler.Delete(id)
}

func (b BoardHandler) ListBoards(ctx context.Context) ([]*backend.Board, error) {
	ids, err := b.handler.List()
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
