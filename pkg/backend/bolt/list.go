package bolt

import (
	"context"

	"github.com/google/uuid"
	bolt "go.etcd.io/bbolt"

	"github.com/twistedogic/orga/pkg/backend"
)

type ListHandler struct {
	handler Store
}

func NewListHandler(db *bolt.DB) (ListHandler, error) {
	h := NewStore(listBucketName, db)
	return ListHandler{
		handler: h,
	}, h.Init()
}

func (l ListHandler) AddList(ctx context.Context, list backend.List) error {
	id := uuid.NewString()
	list.Id = id
	return l.handler.Set(id, &list)
}

func (l ListHandler) GetList(ctx context.Context, id string) (backend.List, error) {
	var list backend.List
	err := l.handler.Get(id, &list)
	return list, err
}

func (l ListHandler) UpdateList(ctx context.Context, list backend.List) error {
	if _, err := l.GetList(ctx, list.Id); err != nil {
		return err
	}
	return l.handler.Set(list.Id, &list)
}

func (l ListHandler) DeleteList(ctx context.Context, id string) error {
	return l.handler.Delete(id)
}

func (l ListHandler) ListLists(ctx context.Context, boardId string) ([]backend.List, error) {
	ids, err := l.handler.List()
	if err != nil {
		return nil, err
	}
	lists := make([]backend.List, 0, len(ids))
	for _, id := range ids {
		list, err := l.GetList(ctx, id)
		if err != nil {
			return nil, err
		}
		if list.BoardId == boardId {
			lists = append(lists, list)
		}
	}
	return lists, nil
}
