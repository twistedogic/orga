package view

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/huh"
	"github.com/charmbracelet/lipgloss"
	"github.com/google/uuid"

	"github.com/twistedogic/orga/pkg/backend"
	"github.com/twistedogic/orga/pkg/config"
)

// State represents the current view state
type State int

const (
	StateBoard State = iota
	StateCardForm
	StateConfirmDelete
)

// Model represents the application state
type Model struct {
	ctx        context.Context
	board      *backend.Board
	lists      []*backend.List
	cards      map[string][]*backend.Card // list ID -> cards
	currentCol int
	currentRow int
	state      State
	width      int
	height     int
	
	// Form state
	cardForm     *huh.Form
	editingCard  *backend.Card
	editingList  *backend.List
	confirmMsg   string
	deleteCard   *backend.Card
	
	// Error state
	err error
}

// New creates a new bubbletea model
func New(ctx context.Context, board *backend.Board) (*Model, error) {
	m := &Model{
		ctx:        ctx,
		board:      board,
		cards:      make(map[string][]*backend.Card),
		currentCol: 0,
		currentRow: 0,
		state:      StateBoard,
	}
	
	if err := m.bootstrap(ctx); err != nil {
		return nil, err
	}
	
	if err := m.loadData(); err != nil {
		return nil, err
	}
	
	return m, nil
}

// bootstrap creates default lists if none exist
func (m *Model) bootstrap(ctx context.Context) error {
	lists, err := m.board.Lists(ctx)
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
	return m.board.AddLists(ctx, lists...)
}

// loadData loads lists and cards from the backend
func (m *Model) loadData() error {
	lists, err := m.board.Lists(m.ctx)
	if err != nil {
		return err
	}
	m.lists = lists
	
	// Load cards for each list
	m.cards = make(map[string][]*backend.Card)
	for _, list := range lists {
		cards, err := list.Cards(m.ctx)
		if err != nil {
			return err
		}
		m.cards[list.Id] = cards
	}
	
	return nil
}

// Init initializes the model
func (m *Model) Init() tea.Cmd {
	return nil
}

// Update handles messages
func (m *Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil
	
	case tea.KeyMsg:
		switch m.state {
		case StateBoard:
			return m.handleBoardKeys(msg)
		case StateCardForm:
			return m.handleFormKeys(msg)
		case StateConfirmDelete:
			return m.handleConfirmKeys(msg)
		}
	}
	
	// Handle form updates
	if m.state == StateCardForm && m.cardForm != nil {
		form, cmd := m.cardForm.Update(msg)
		if f, ok := form.(*huh.Form); ok {
			m.cardForm = f
		}
		return m, cmd
	}
	
	return m, nil
}

// handleBoardKeys handles keyboard input for the board view
func (m *Model) handleBoardKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "q", "ctrl+c":
		return m, tea.Quit
	case "left", "h":
		m.moveLeft()
	case "right", "l":
		m.moveRight()
	case "up", "k":
		m.moveUp()
	case "down", "j":
		m.moveDown()
	case "n":
		m.startNewCard()
		return m, m.cardForm.Init()
	case "enter":
		m.startEditCard()
		return m, m.cardForm.Init()
	case "d":
		m.startDeleteCard()
	case "r":
		m.loadData()
	}
	return m, nil
}

// handleFormKeys handles keyboard input for the form view
func (m *Model) handleFormKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "esc":
		m.state = StateBoard
		return m, nil
	}
	
	// Check if form is complete
	if m.cardForm.State == huh.StateCompleted {
		m.saveCard()
		m.state = StateBoard
		return m, nil
	}
	
	return m, nil
}

// handleConfirmKeys handles keyboard input for the confirmation dialog
func (m *Model) handleConfirmKeys(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "y", "Y":
		m.confirmDelete()
		m.state = StateBoard
	case "n", "N", "esc":
		m.state = StateBoard
	}
	return m, nil
}

// Navigation methods
func (m *Model) moveLeft() {
	if m.currentCol > 0 {
		m.currentCol--
		m.currentRow = 0
	}
}

func (m *Model) moveRight() {
	if m.currentCol < len(m.lists)-1 {
		m.currentCol++
		m.currentRow = 0
	}
}

func (m *Model) moveUp() {
	if m.currentRow > 0 {
		m.currentRow--
	}
}

func (m *Model) moveDown() {
	if m.currentCol < len(m.lists) {
		list := m.lists[m.currentCol]
		cards := m.cards[list.Id]
		if m.currentRow < len(cards)-1 {
			m.currentRow++
		}
	}
}

// Card management methods
func (m *Model) startNewCard() {
	if m.currentCol >= len(m.lists) {
		return
	}
	
	m.editingCard = nil
	m.editingList = m.lists[m.currentCol]
	m.state = StateCardForm
	m.createCardForm("", "", 0, 0)
}

func (m *Model) startEditCard() {
	if m.currentCol >= len(m.lists) {
		return
	}
	
	list := m.lists[m.currentCol]
	cards := m.cards[list.Id]
	
	if m.currentRow >= len(cards) {
		return
	}
	
	card := cards[m.currentRow]
	m.editingCard = card
	m.editingList = list
	m.state = StateCardForm
	m.createCardForm(card.Name, card.Description, card.Value, card.Effort)
}

func (m *Model) startDeleteCard() {
	if m.currentCol >= len(m.lists) {
		return
	}
	
	list := m.lists[m.currentCol]
	cards := m.cards[list.Id]
	
	if m.currentRow >= len(cards) {
		return
	}
	
	card := cards[m.currentRow]
	m.deleteCard = card
	m.confirmMsg = fmt.Sprintf("Delete card '%s'?", card.Name)
	m.state = StateConfirmDelete
}

func (m *Model) createCardForm(name, description string, value, effort int) {
	valueStr := ""
	effortStr := ""
	if value > 0 {
		valueStr = strconv.Itoa(value)
	}
	if effort > 0 {
		effortStr = strconv.Itoa(effort)
	}
	
	// Create form fields with proper key references
	nameField := huh.NewInput().
		Key("name").
		Title("Card Name").
		Value(&name).
		Validate(func(s string) error {
			if strings.TrimSpace(s) == "" {
				return fmt.Errorf("name is required")
			}
			return nil
		})
	
	descField := huh.NewText().
		Key("description").
		Title("Description").
		Value(&description)
	
	valueField := huh.NewInput().
		Key("value").
		Title("Value").
		Value(&valueStr).
		Validate(func(s string) error {
			if s == "" {
				return nil
			}
			if _, err := strconv.Atoi(s); err != nil {
				return fmt.Errorf("value must be a number")
			}
			return nil
		})
	
	effortField := huh.NewInput().
		Key("effort").
		Title("Effort").
		Value(&effortStr).
		Validate(func(s string) error {
			if s == "" {
				return nil
			}
			if _, err := strconv.Atoi(s); err != nil {
				return fmt.Errorf("effort must be a number")
			}
			return nil
		})
	
	m.cardForm = huh.NewForm(
		huh.NewGroup(nameField, descField, valueField, effortField),
	)
}

func (m *Model) saveCard() {
	if m.cardForm == nil || m.editingList == nil {
		return
	}
	
	// Extract form values using the proper keys
	nameInput := m.cardForm.GetString("name")
	descInput := m.cardForm.GetString("description")
	valueInput := m.cardForm.GetString("value")
	effortInput := m.cardForm.GetString("effort")
	
	if strings.TrimSpace(nameInput) == "" {
		return
	}
	
	value := 0
	effort := 0
	
	if valueInput != "" {
		value, _ = strconv.Atoi(valueInput)
	}
	if effortInput != "" {
		effort, _ = strconv.Atoi(effortInput)
	}
	
	if m.editingCard == nil {
		// Create new card
		newCard := &backend.Card{
			Id:          uuid.New().String(),
			Name:        nameInput,
			Description: descInput,
			Value:       value,
			Effort:      effort,
			ListId:      m.editingList.Id,
		}
		newCard.SetBackend(m.board.GetBackend())
		if err := m.board.GetBackend().AddCard(m.ctx, newCard); err == nil {
			m.loadData()
		}
	} else {
		// Update existing card
		m.editingCard.Name = nameInput
		m.editingCard.Description = descInput
		m.editingCard.Value = value
		m.editingCard.Effort = effort
		m.editingCard.SetBackend(m.board.GetBackend())
		if err := m.editingCard.Update(m.ctx); err == nil {
			m.loadData()
		}
	}
}

func (m *Model) confirmDelete() {
	if m.deleteCard == nil {
		return
	}
	
	if err := m.board.GetBackend().DeleteCard(m.ctx, m.deleteCard.Id); err == nil {
		m.loadData()
	}
}

// View renders the current view
func (m *Model) View() string {
	switch m.state {
	case StateBoard:
		return m.viewBoard()
	case StateCardForm:
		return m.viewCardForm()
	case StateConfirmDelete:
		return m.viewConfirmDelete()
	}
	return ""
}

func (m *Model) viewBoard() string {
	if len(m.lists) == 0 {
		return "No lists found"
	}
	
	// Styles
	titleStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("15")).
		Background(lipgloss.Color("63")).
		Padding(0, 1)
	
	listStyle := lipgloss.NewStyle().
		Border(lipgloss.NormalBorder()).
		BorderForeground(lipgloss.Color("238")).
		Padding(1).
		Width(25).
		Height(m.height - 8)
	
	selectedListStyle := listStyle.Copy().
		BorderForeground(lipgloss.Color("12"))
	
	cardStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("240")).
		Padding(0, 1).
		Margin(0, 0, 1, 0)
	
	selectedCardStyle := cardStyle.Copy().
		BorderForeground(lipgloss.Color("12")).
		Background(lipgloss.Color("18"))
	
	// Build title
	title := titleStyle.Render(fmt.Sprintf(" %s ", m.board.Name))
	
	// Build lists
	var lists []string
	for i, list := range m.lists {
		var cards []string
		listCards := m.cards[list.Id]
		
		if len(listCards) == 0 {
			cards = append(cards, lipgloss.NewStyle().
				Foreground(lipgloss.Color("240")).
				Render("(empty)\nPress 'n' to add a new card"))
		} else {
			for j, card := range listCards {
				cardText := card.Name
				if card.Description != "" {
					cardText += "\n" + lipgloss.NewStyle().
						Foreground(lipgloss.Color("240")).
						Render(card.Description)
				}
				if card.Value > 0 || card.Effort > 0 {
					cardText += "\n" + lipgloss.NewStyle().
						Foreground(lipgloss.Color("33")).
						Render(fmt.Sprintf("V:%d E:%d", card.Value, card.Effort))
				}
				
				style := cardStyle
				if i == m.currentCol && j == m.currentRow {
					style = selectedCardStyle
				}
				cards = append(cards, style.Render(cardText))
			}
		}
		
		listContent := strings.Join(cards, "\n")
		listTitle := lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("15")).
			Render(list.Name)
		
		style := listStyle
		if i == m.currentCol {
			style = selectedListStyle
		}
		
		lists = append(lists, style.Render(listTitle+"\n\n"+listContent))
	}
	
	// Build footer
	footer := lipgloss.NewStyle().
		Foreground(lipgloss.Color("240")).
		Render("Navigation: ←→ Move between lists | ↑↓ Move between cards | Enter Edit card | n New card | d Delete card | r Refresh | q Quit")
	
	// Layout
	board := lipgloss.JoinHorizontal(lipgloss.Top, lists...)
	
	return lipgloss.JoinVertical(
		lipgloss.Center,
		title,
		"",
		board,
		"",
		footer,
	)
}

func (m *Model) viewCardForm() string {
	if m.cardForm == nil {
		return "Loading form..."
	}
	
	title := "New Card"
	if m.editingCard != nil {
		title = "Edit Card"
	}
	
	titleStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("15")).
		Background(lipgloss.Color("63")).
		Padding(0, 1)
	
	return lipgloss.JoinVertical(
		lipgloss.Left,
		titleStyle.Render(fmt.Sprintf(" %s ", title)),
		"",
		m.cardForm.View(),
		"",
		lipgloss.NewStyle().
			Foreground(lipgloss.Color("240")).
			Render("Press Esc to cancel"),
	)
}

func (m *Model) viewConfirmDelete() string {
	titleStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("15")).
		Background(lipgloss.Color("124")).
		Padding(0, 1)
	
	return lipgloss.JoinVertical(
		lipgloss.Center,
		titleStyle.Render(" Confirm Delete "),
		"",
		m.confirmMsg,
		"",
		"Press Y to confirm, N to cancel",
	)
}

// Run starts the bubbletea program
func (m *Model) Run() error {
	p := tea.NewProgram(m, tea.WithAltScreen())
	_, err := p.Run()
	return err
}


