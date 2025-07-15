# Orga - Terminal UI Kanban Board

A local command-line Kanban board application with Terminal User Interface (TUI) for managing tasks in an agile workflow. All data is stored locally using BoltDB - no external services required.

## Features

- **TUI Kanban Board**: Visual board with columns for different workflow stages
- **Card Management**: Create, edit, and delete cards with name, description, value, and effort
- **Keyboard Navigation**: Intuitive keyboard controls for navigating the board
- **Local Data Storage**: Uses BoltDB for fast, local data persistence
- **Multiple Boards**: Support for multiple boards with custom names
- **Offline First**: Works completely offline with no external dependencies

## Installation

```bash
go build -o orga
```

## Usage

### Running the TUI

```bash
./orga run
```

#### Options

- `--board, -b`: Specify board name (default: "Main Board")
- `--db, -d`: Specify database file path (default: "orga.db")

## TUI Controls

### Navigation

- **← →**: Move between lists (columns)
- **↑ ↓**: Move between cards within a list
- **Enter**: Edit the selected card
- **n**: Create a new card in the current list
- **d**: Delete the selected card
- **r**: Refresh the board
- **q** or **Ctrl+C**: Quit the application

### Default Lists

The application creates these default lists when first run:

1. TODO
2. READY TO DEVELOPMENT  
3. DOING
4. TESTING
5. DONE

### Card Fields

Each card can have:

- **Name**: Card title (required)
- **Description**: Detailed description
- **Value**: Business value (numeric)
- **Effort**: Development effort estimate (numeric)

Cards are automatically sorted by priority (higher value, lower effort first).

## Architecture

- **Backend**: Pluggable backend system with BoltDB implementation
- **View**: TUI implementation using tview library
- **CLI**: Command-line interface using urfave/cli

## Dependencies

- `github.com/rivo/tview`: Terminal UI library
- `go.etcd.io/bbolt`: Embedded key/value database
- `github.com/urfave/cli/v2`: CLI framework
- `github.com/google/uuid`: UUID generation

## Example Workflow

1. Start the application: `./orga run`
2. Navigate to the "TODO" list using arrow keys
3. Press `n` to create a new card
4. Fill in the card details and save
5. Use arrow keys to select the card
6. Press `Enter` to edit or `d` to delete
7. Navigate between lists to move through your workflow

## Future Enhancements

- Drag and drop functionality for moving cards between lists
- Custom list configuration
- Card assignment and due dates
- Search and filtering capabilities
- Export/import functionality for data backup
- Multiple board templates