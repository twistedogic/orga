package testutil

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/twistedogic/orga/pkg/backend"
)

func Ok(t *testing.T, msg string, err error) {
	t.Helper()
	if err != nil {
		t.Fatalf("%s: %v", msg, err)
	}
}

type item struct {
	Name string
	Id   string
}

func boardsToItems(boards []*backend.Board) []item {
	items := make([]item, len(boards))
	for i, b := range boards {
		items[i] = item{Name: b.Name, Id: b.Id}
	}
	return items
}

func listsToItems(lists []*backend.List) []item {
	items := make([]item, len(lists))
	for i, l := range lists {
		items[i] = item{Name: l.Name, Id: l.Id}
	}
	return items
}

func cardsToItems(cards []*backend.Card) []item {
	items := make([]item, len(cards))
	for i, c := range cards {
		items[i] = item{Name: c.Name, Id: c.Id}
	}
	return items
}

func findId(t *testing.T, items []item, name string) string {
	t.Helper()
	for _, item := range items {
		if item.Name == name {
			return item.Id
		}
	}
	t.Fatalf("no found with name %s in %v", name, items)
	return ""
}

func shouldNotFindId(t *testing.T, items []item, name string) {
	t.Helper()
	for _, item := range items {
		if item.Name == name {
			t.Fatalf("should not found with name %s in %v", name, items)
		}
	}
}

func equalStrings(t *testing.T, a, b string) {
	t.Helper()
	if a != b {
		t.Fatalf("want: %v, got: %v", a, b)
	}
}

func TestBackend(t *testing.T, b backend.Backend) {
	ctx := context.TODO()
	now := time.Now().Unix()
	boardName := fmt.Sprintf("Board-%d", now)
	board := &backend.Board{Name: boardName}
	Ok(t, "add board", b.AddBoard(ctx, board))
	boards, err := b.ListBoards(ctx)
	Ok(t, "list boards", err)
	boardId := findId(t, boardsToItems(boards), boardName)
	board, err = b.GetBoard(ctx, boardId)
	Ok(t, "get board", err)

	listName := fmt.Sprintf("List-%d", now)
	list := &backend.List{Name: listName, BoardId: boardId}
	Ok(t, "add list", board.AddLists(ctx, list))
	lists, err := board.Lists(ctx)
	Ok(t, "list lists", err)
	listId := findId(t, listsToItems(lists), listName)
	list, err = b.GetList(ctx, listId)
	Ok(t, "get list", err)

	cardName := fmt.Sprintf("Card-%d", now)
	card := &backend.Card{Name: cardName, ListId: listId}
	Ok(t, "add card", list.AddCards(ctx, card))
	cards, err := list.Cards(ctx)
	Ok(t, "list cards", err)
	cardId := findId(t, cardsToItems(cards), cardName)
	card, err = b.GetCard(ctx, cardId)
	Ok(t, "get card", err)

	card.Name = fmt.Sprintf("%s-new", cardName)
	Ok(t, "update card", b.UpdateCard(ctx, card))
	newCard, err := b.GetCard(ctx, cardId)
	Ok(t, "get card", err)
	equalStrings(t, card.Name, newCard.Name)

	list.Name = fmt.Sprintf("%s-new", listName)
	Ok(t, "update list", b.UpdateList(ctx, list))
	newList, err := b.GetList(ctx, listId)
	Ok(t, "get list", err)
	equalStrings(t, list.Name, newList.Name)

	board.Name = fmt.Sprintf("%s-new", boardName)
	Ok(t, "update board", b.UpdateBoard(ctx, board))
	newBoard, err := b.GetBoard(ctx, boardId)
	Ok(t, "get board", err)
	equalStrings(t, board.Name, newBoard.Name)

	list2 := &backend.List{Name: fmt.Sprintf("%s-2", listName), BoardId: boardId}
	Ok(t, "add list2", board.AddLists(ctx, list2))
	lists, err = board.Lists(ctx)
	Ok(t, "list lists", err)
	list2Id := findId(t, listsToItems(lists), list2.Name)
	card.ListId = list2Id
	Ok(t, "update card", card.Update(ctx))
	cards, err = b.ListCards(ctx, list2Id)
	Ok(t, "list cards", err)
	findId(t, cardsToItems(cards), card.Name)
	cards, err = list.Cards(ctx)
	Ok(t, "list cards", err)
	shouldNotFindId(t, cardsToItems(cards), card.Name)

	Ok(t, "delete card", b.DeleteCard(ctx, cardId))
	Ok(t, "delete list", b.DeleteList(ctx, listId))
	Ok(t, "delete list2", b.DeleteList(ctx, list2Id))
	Ok(t, "delete board", b.DeleteBoard(ctx, boardId))
}
