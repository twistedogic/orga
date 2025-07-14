package bolt

import (
	"context"

	"github.com/google/uuid"

	"github.com/twistedogic/orga/pkg/backend"
)

func (b Backend) AddList(ctx context.Context, list *backend.List) error {
	id := uuid.NewString()
	list.Id = id
	return b.ListHandler.Set(id, list)
}

func (b Backend) GetList(ctx context.Context, id string) (*backend.List, error) {
	list := new(backend.List)
	if err := b.ListHandler.Get(id, list); err != nil {
		return nil, err
	}
	list.SetBackend(b)
	return list, nil
}

func (b Backend) UpdateList(ctx context.Context, list *backend.List) error {
	if _, err := b.GetList(ctx, list.Id); err != nil {
		return err
	}
	return b.ListHandler.Set(list.Id, list)
}

func (b Backend) DeleteList(ctx context.Context, id string) error {
	return b.ListHandler.Delete(id)
}

func (b Backend) ListLists(ctx context.Context, boardId string) ([]*backend.List, error) {
	ids, err := b.ListHandler.List()
	if err != nil {
		return nil, err
	}
	lists := make([]*backend.List, 0, len(ids))
	for _, id := range ids {
		list, err := b.GetList(ctx, id)
		if err != nil {
			return nil, err
		}
		if list.BoardId == boardId {
			lists = append(lists, list)
		}
	}
	return lists, nil
}
