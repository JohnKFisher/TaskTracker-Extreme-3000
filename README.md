# TaskTracker Extreme 3000

A personal sidebar task manager for Windows with Desk365 helpdesk integration. Built for my own workflow. Outside usefulness is incidental, and no support, stability guarantees, or warranty of any kind is implied.

## What This Is

A largely vibe-coded personal hobby app that lives on the right edge of my screen. It keeps my tasks, in-progress work, and open support tickets visible at all times without taking up a full window. It's an Electron app that runs in the system tray and reserves screen space so maximized windows don't cover it.

## What It Does

- Kanban-style task board with columns: Standing, Priority, In Progress, To-Do, Rainy Day, Done
- Drag-and-drop reordering within and between columns
- Global shortcut (`Ctrl+Shift+T`) to show or hide the sidebar
- Quick-add overlay (`Ctrl+Shift+N`) to capture a task without switching windows
- Desk365 ticket integration — shows your open/unresolved tickets, auto-refreshes every 5 minutes
- Persistent notes tab for scratch text
- Minimizes to system tray instead of closing

## Data And Privacy

All data is stored locally on your machine. Nothing is sent anywhere except outbound API calls to your own Desk365 instance. No telemetry, no analytics, no cloud sync.

When packaged, data is stored in your Electron `userData` folder (`%AppData%\tasktracker-extreme-3000`). When running from source, data is stored in a `data/` folder in the project directory. The `data/` folder is gitignored and will never be committed.

Your Desk365 API key is stored in `data/config.json` on your local machine only.

## Configuration

Copy `config.example.json` into your `data/` folder and rename it `config.json`:

```json
{
  "apiKey": "your-desk365-api-key-here",
  "desk365Domain": "yourcompany.desk365.io"
}
```

The app will also prompt you for the API key on first run if `config.json` is missing or has no key.

## Running From Source

Requires Node.js and npm.

```bash
npm install
npm start
```

## Building

Produces a portable Windows executable in `dist/`:

```bash
npm run build
```

## Migrating From An Older Install

If you previously ran a version that stored data in a hardcoded OneDrive path, the app will automatically copy your existing data files to the new location on first launch. No data loss should occur.

## Limitations

- Windows only — relies on Windows AppBar APIs for screen reservation
- Requires a Desk365 account for ticket integration (the task board works without it)
- Tested on my machine; your mileage may vary
