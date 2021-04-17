package bolt

import (
	"context"

	"github.com/google/uuid"

	"github.com/twistedogic/orga/pkg/backend"
)

func (b Backend) AddCard(ctx context.Context, card *backend.Card) error {
	id := uuid.NewString()
	card.Id = id
	return b.CardHandler.Set(id, &card)
}

func (b Backend) GetCard(ctx context.Context, id string) (*backend.Card, error) {
	card := new(backend.Card)
	if err := b.CardHandler.Get(id, card); err != nil {
		return nil, err
	}
	card.SetBackend(b)
	return card, nil
}

func (b Backend) UpdateCard(ctx context.Context, card *backend.Card) error {
	if _, err := b.GetCard(ctx, card.Id); err != nil {
		return err
	}
	return b.CardHandler.Set(card.Id, card)
}

func (b Backend) DeleteCard(ctx context.Context, id string) error {
	return b.CardHandler.Delete(id)
}

func (b Backend) ListCards(ctx context.Context, listId string) ([]*backend.Card, error) {
	ids, err := b.CardHandler.List()
	if err != nil {
		return nil, err
	}
	cards := make([]*backend.Card, 0, len(ids))
	for _, id := range ids {
		card, err := b.GetCard(ctx, id)
		if err != nil {
			return nil, err
		}
		if card.ListId == listId {
			cards = append(cards, card)
		}
	}
	return cards, nil
}
