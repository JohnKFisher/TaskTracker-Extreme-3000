# TaskTracker Extreme 3000

A personal sidebar task manager for Windows and macOS with Desk365 helpdesk integration. Built for my own workflow. Outside usefulness is incidental, and no support, stability guarantees, or warranty of any kind is implied.

## What This Is

A largely vibe-coded personal hobby app that lives on the right edge of your screen. It keeps tasks, in-progress work, and open support tickets visible at all times without taking up a full window. Built with [Tauri v2](https://tauri.app) — native WebView, ~8MB binary, no Electron.

## What It Does

- Kanban-style task board with columns: Standing, Priority, In Progress, To-Do, Rainy Day, Done
- Drag-and-drop reordering within and between columns
- Global shortcut (`Ctrl/Cmd+Shift+T`) to show or hide the sidebar
- Quick-add overlay (`Ctrl/Cmd+Shift+N`) to capture a task without switching windows
- Desk365 ticket integration — shows your open/unresolved tickets, auto-refreshes every 5 minutes
- Persistent notes tab for scratch text
- Minimizes to system tray instead of closing
- Settings tab to configure a sync folder for keeping data in sync across machines

## Sync Between Machines

The app supports an optional sync folder. Point it at any cloud-synced location (OneDrive, iCloud Drive, Dropbox, etc.) and your tasks, notes, and config will be shared across machines. Window position stays machine-specific.

Configure it in the **Settings** tab (gear icon) inside the app.

## Data And Privacy

All data is stored locally on your machine in JSON files. Nothing is sent anywhere except outbound API calls to your own Desk365 instance. No telemetry, no analytics.

When packaged, local data is stored in your OS app-data folder:
- **Windows:** `%AppData%\com.tasktracker.extreme3000\`
- **macOS:** `~/Library/Application Support/com.tasktracker.extreme3000/`

If a sync folder is configured, task/note/config data is stored there instead. The `data/` project folder and `local-settings.json` are gitignored and will never be committed.

## First-Run Setup

On first launch the Tickets tab will prompt you for your Desk365 domain and API key. Your API key is stored locally in `config.json` — never committed to the repo.

## Getting Builds

GitHub Actions builds installers automatically on every push to `main`. No local Rust installation needed.

Download from: **GitHub → Actions → (latest run) → Artifacts**

| Artifact | Platform |
|---|---|
| `tasktracker-extreme-3000-windows-x64` | Windows installer |
| `tasktracker-extreme-3000-macos-apple-silicon` | macOS M1/M2/M3 DMG |
| `tasktracker-extreme-3000-macos-intel` | macOS Intel DMG |

> **First-run warning:** Builds are unsigned. On macOS: right-click the app → Open → Open anyway. On Windows: SmartScreen → "More info" → "Run anyway". One-time prompt.

## Building From Source

Requires [Rust](https://rustup.rs) and Node.js.

```bash
npm install
npm run dev    # hot-reload dev mode
npm run build  # production build
```

## Limitations

- Requires a Desk365 account for ticket integration (the task board works without it)
- Tested on my machines; your mileage may vary
