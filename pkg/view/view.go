package view

import (
	"context"
	"fmt"
	"strconv"

	"github.com/gdamore/tcell/v2"
	"github.com/google/uuid"
	"github.com/rivo/tview"

	"github.com/twistedogic/orga/pkg/backend"
	"github.com/twistedogic/orga/pkg/config"
)

type View struct {
	context.Context
	*tview.Application
	*backend.Board

	// UI components
	grid       *tview.Grid
	lists      []*backend.List
	listViews  []*tview.List
	currentCol int
	footer     *tview.TextView
}

func New(ctx context.Context, board *backend.Board) (View, error) {
	v := View{
		Context:     ctx,
		Application: tview.NewApplication(),
		Board:       board,
		currentCol:  0,
	}
	err := v.Init(ctx)
	return v, err
}

func (v *View) bootstrap(ctx context.Context) error {
	lists, err := v.Lists(ctx)
	if err != nil {
		return err
	}
	if len(lists) != 0 {
		return nil
	}
	for i, l := range config.DefaultList {
		lists = append(lists, &backend.List{
			Id:   uuid.New().String(),
			Name: l,
			Pos:  float64(i),
		})
	}
	return v.AddLists(ctx, lists...)
}

func (v *View) Init(ctx context.Context) error {
	if err := v.bootstrap(ctx); err != nil {
		return err
	}
	return v.buildUI(ctx)
}

func (v *View) buildUI(ctx context.Context) error {
	// Get lists from backend
	lists, err := v.Lists(ctx)
	if err != nil {
		return err
	}
	v.lists = lists

	// Create grid layout
	v.grid = tview.NewGrid()
	v.grid.SetBorder(true).SetTitle(fmt.Sprintf(" %s ", v.Board.Name))

	// Create footer
	v.footer = tview.NewTextView().
		SetText("Navigation: ←→ Move between lists | ↑↓ Move between cards | Enter Edit card | n New card | d Delete card | q Quit").
		SetTextAlign(tview.AlignCenter).
		SetDynamicColors(true)

	// Create list views
	v.listViews = make([]*tview.List, len(lists))
	for i, list := range lists {
		v.listViews[i] = v.createListView(ctx, list, i)
	}

	// Set up grid layout
	v.setupLayout()

	// Set up key bindings
	v.setupKeyBindings()

	// Highlight first column
	v.highlightColumn(0)

	return nil
}

func (v *View) createListView(ctx context.Context, list *backend.List, index int) *tview.List {
	listView := tview.NewList()
	listView.ShowSecondaryText(true).
		SetBorder(true).
		SetTitle(fmt.Sprintf(" %s ", list.Name))

	// Load cards for this list
	if err := v.loadCards(ctx, listView, list); err != nil {
		// Show error in list
		listView.AddItem("Error loading cards", err.Error(), 0, nil)
	}

	return listView
}

func (v *View) loadCards(ctx context.Context, listView *tview.List, list *backend.List) error {
	cards, err := list.Cards(ctx)
	if err != nil {
		return err
	}

	listView.Clear()
	for _, card := range cards {
		primary := card.Name
		secondary := ""
		if card.Description != "" {
			secondary = card.Description
		}
		if card.Value > 0 || card.Effort > 0 {
			secondary += fmt.Sprintf(" [Value:%d Effort:%d]", card.Value, card.Effort)
		}
		
		listView.AddItem(primary, secondary, 0, func() {
			v.editCard(ctx, card)
		})
	}

	if listView.GetItemCount() == 0 {
		listView.AddItem("(empty)", "Press 'n' to add a new card", 0, nil)
	}

	return nil
}

func (v *View) setupLayout() {
	numCols := len(v.lists)
	if numCols == 0 {
		return
	}

	// Calculate column widths
	colWidth := 100 / numCols

	// Set up grid with dynamic columns
	v.grid.SetRows(0, 3) // Main area and footer
	cols := make([]int, numCols)
	for i := range cols {
		cols[i] = colWidth
	}
	v.grid.SetColumns(cols...)

	// Add list views to grid
	for i, listView := range v.listViews {
		v.grid.AddItem(listView, 0, i, 1, 1, 0, 0, false)
	}

	// Add footer
	v.grid.AddItem(v.footer, 1, 0, 1, numCols, 0, 0, false)

	v.SetRoot(v.grid, true)
}

func (v *View) setupKeyBindings() {
	v.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		switch event.Key() {
		case tcell.KeyEscape, tcell.KeyCtrlC:
			v.Stop()
			return nil
		case tcell.KeyRune:
			switch event.Rune() {
			case 'q':
				v.Stop()
				return nil
			case 'n':
				v.createNewCard()
				return nil
			case 'd':
				v.deleteCurrentCard()
				return nil
			case 'r':
				v.refreshBoard()
				return nil
			}
		case tcell.KeyLeft:
			v.moveLeft()
			return nil
		case tcell.KeyRight:
			v.moveRight()
			return nil
		case tcell.KeyEnter:
			v.editCurrentCard()
			return nil
		}
		return event
	})
}

func (v *View) highlightColumn(col int) {
	if col < 0 || col >= len(v.listViews) {
		return
	}

	// Remove focus from all columns
	for i, listView := range v.listViews {
		if i == col {
			listView.SetBorderColor(tcell.ColorYellow)
			v.SetFocus(listView)
		} else {
			listView.SetBorderColor(tcell.ColorWhite)
		}
	}
	v.currentCol = col
}

func (v *View) moveLeft() {
	if v.currentCol > 0 {
		v.highlightColumn(v.currentCol - 1)
	}
}

func (v *View) moveRight() {
	if v.currentCol < len(v.listViews)-1 {
		v.highlightColumn(v.currentCol + 1)
	}
}

func (v *View) createNewCard() {
	if v.currentCol >= len(v.lists) {
		return
	}

	v.showCardForm(context.Background(), nil, v.lists[v.currentCol])
}

func (v *View) editCurrentCard() {
	if v.currentCol >= len(v.listViews) {
		return
	}

	listView := v.listViews[v.currentCol]
	currentIndex := listView.GetCurrentItem()
	if currentIndex < 0 {
		return
	}

	// Get the card from the backend
	list := v.lists[v.currentCol]
	cards, err := list.Cards(context.Background())
	if err != nil || currentIndex >= len(cards) {
		return
	}

	v.showCardForm(context.Background(), cards[currentIndex], list)
}

func (v *View) editCard(ctx context.Context, card *backend.Card) {
	list, err := card.List(ctx)
	if err != nil {
		return
	}
	v.showCardForm(ctx, card, list)
}

func (v *View) deleteCurrentCard() {
	if v.currentCol >= len(v.listViews) {
		return
	}

	listView := v.listViews[v.currentCol]
	currentIndex := listView.GetCurrentItem()
	if currentIndex < 0 {
		return
	}

	// Get the card from the backend
	list := v.lists[v.currentCol]
	cards, err := list.Cards(context.Background())
	if err != nil || currentIndex >= len(cards) {
		return
	}

	card := cards[currentIndex]
	
	// Show confirmation dialog
	modal := tview.NewModal().
		SetText(fmt.Sprintf("Delete card '%s'?", card.Name)).
		AddButtons([]string{"Delete", "Cancel"}).
		SetDoneFunc(func(buttonIndex int, buttonLabel string) {
			if buttonLabel == "Delete" {
				// Delete the card through the board's backend
				if err := v.Board.GetBackend().DeleteCard(context.Background(), card.Id); err == nil {
					v.refreshBoard()
				}
			}
			v.SetRoot(v.grid, true)
		})

	v.SetRoot(modal, false)
}

func (v *View) showCardForm(ctx context.Context, card *backend.Card, list *backend.List) {
	form := tview.NewForm()
	
	var name, description string
	var value, effort int
	
	if card != nil {
		name = card.Name
		description = card.Description
		value = card.Value
		effort = card.Effort
	}

	form.AddInputField("Name", name, 50, nil, func(text string) {
		name = text
	}).
	AddInputField("Description", description, 50, nil, func(text string) {
		description = text
	}).
	AddInputField("Value", strconv.Itoa(value), 10, func(textToCheck string, lastChar rune) bool {
		_, err := strconv.Atoi(textToCheck)
		return err == nil || textToCheck == ""
	}, func(text string) {
		if text == "" {
			value = 0
		} else {
			value, _ = strconv.Atoi(text)
		}
	}).
	AddInputField("Effort", strconv.Itoa(effort), 10, func(textToCheck string, lastChar rune) bool {
		_, err := strconv.Atoi(textToCheck)
		return err == nil || textToCheck == ""
	}, func(text string) {
		if text == "" {
			effort = 0
		} else {
			effort, _ = strconv.Atoi(text)
		}
	}).
	AddButton("Save", func() {
		if name == "" {
			return
		}

		if card == nil {
			// Create new card
			newCard := &backend.Card{
				Id:          uuid.New().String(),
				Name:        name,
				Description: description,
				Value:       value,
				Effort:      effort,
				ListId:      list.Id,
			}
			newCard.SetBackend(v.Board.GetBackend())
			if err := v.Board.GetBackend().AddCard(ctx, newCard); err == nil {
				v.refreshBoard()
			}
		} else {
			// Update existing card
			card.Name = name
			card.Description = description
			card.Value = value
			card.Effort = effort
			card.SetBackend(v.Board.GetBackend())
			if err := card.Update(ctx); err == nil {
				v.refreshBoard()
			}
		}

		v.SetRoot(v.grid, true)
	}).
	AddButton("Cancel", func() {
		v.SetRoot(v.grid, true)
	})

	form.SetBorder(true).SetTitle(" Card Details ")
	v.SetRoot(form, true)
}

func (v *View) refreshBoard() {
	for i, list := range v.lists {
		if i < len(v.listViews) {
			v.loadCards(context.Background(), v.listViews[i], list)
		}
	}
}

func (v *View) Run() error {
	return v.Application.Run()
}
