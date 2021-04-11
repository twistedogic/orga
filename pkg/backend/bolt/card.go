package bolt

import (
	"context"

	"github.com/google/uuid"
	bolt "go.etcd.io/bbolt"

	"github.com/twistedogic/orga/pkg/backend"
)

type CardHandler struct {
	handler Store
}

func NewCardHandler(db *bolt.DB) (CardHandler, error) {
	h := NewStore(cardBucketName, db)
	return CardHandler{
		handler: h,
	}, h.Init()
}

func (c CardHandler) AddCard(ctx context.Context, card backend.Card) error {
	id := uuid.NewString()
	card.Id = id
	return c.handler.Set(id, &card)
}

func (c CardHandler) GetCard(ctx context.Context, id string) (backend.Card, error) {
	var card backend.Card
	err := c.handler.Get(id, &card)
	return card, err
}

func (c CardHandler) UpdateCard(ctx context.Context, card backend.Card) error {
	if _, err := c.GetCard(ctx, card.Id); err != nil {
		return err
	}
	return c.handler.Set(card.Id, &card)
}

func (c CardHandler) DeleteCard(ctx context.Context, id string) error {
	return c.handler.Delete(id)
}

func (c CardHandler) ListCards(ctx context.Context, listId string) ([]backend.Card, error) {
	ids, err := c.handler.List()
	if err != nil {
		return nil, err
	}
	cards := make([]backend.Card, 0, len(ids))
	for _, id := range ids {
		card, err := c.GetCard(ctx, id)
		if err != nil {
			return nil, err
		}
		if card.ListId == listId {
			cards = append(cards, card)
		}
	}
	return cards, nil
}
