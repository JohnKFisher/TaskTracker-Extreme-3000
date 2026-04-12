# TaskTracker Extreme 3000

A personal sidebar task manager for Windows and macOS with Desk365 helpdesk integration. Built for my own workflow. Outside usefulness is incidental, and no support, stability guarantees, or warranty of any kind is implied.

GitHub: [JohnKFisher/TaskTracker-Extreme-3000](https://github.com/JohnKFisher/TaskTracker-Extreme-3000)

## What This Is

A largely vibe-coded personal hobby app that lives on the edge of the screen and keeps tasks, notes, and support work visible without turning into a full desktop suite. It is built with [Tauri v2](https://tauri.app) for a lightweight native shell and a plain HTML/CSS/JS renderer.

Windows is the primary UX tie-breaker for the project. macOS remains a supported target, but when platform conventions differ, Windows behavior wins unless explicitly documented otherwise.

## What It Does

- Kanban-style task board with columns: Standing, Priority, In Progress, To-Do, Rainy Day, Done
- Drag-and-drop reordering within and between columns
- Global shortcut (`Ctrl/Cmd+Shift+T`) to show or hide the sidebar
- Quick-add overlay (`Ctrl/Cmd+Shift+N`) to capture a task without switching windows
- Desk365 ticket integration with secure API-key storage and periodic refresh
- Persistent notes tab for scratch text
- Windows tray behavior, while macOS window close saves and quits the app
- Settings tab with sync-folder controls, storage status, and About info

## Data, Privacy, And Storage

All user data stays local unless you explicitly connect your own Desk365 account. There is no telemetry, analytics, or hidden network activity.

When packaged, local data is stored in the OS app-data folder:
- **Windows:** `%AppData%\com.tasktracker.extreme3000\`
- **macOS:** `~/Library/Application Support/com.tasktracker.extreme3000/`

Shared-data files are plain JSON:
- `tasks.json`
- `notes.json`
- `config.json` (Desk365 hostname only, no secret)
- `hidden-tickets.json`

Machine-specific data stays local and is never synced:
- `local-settings.json`
- `window-state.json`

Desk365 API keys are **not** stored in `config.json`. They are stored in the operating system’s secure credential store.

If you configure a sync folder, shared data is written there instead of the local app-data folder. If that folder later becomes unavailable, the app will pause shared-data access and show a warning instead of silently falling back to a different location.

## First-Run Desk365 Setup

The Tickets tab will ask for:
- your Desk365 hostname, stored in `config.json`
- your Desk365 API key, stored securely in the OS credential store

Enter the hostname only, for example `yourcompany.desk365.io`. Do not include `https://` or a path.

## Versioning

Version/build metadata now comes from the checked-in `version.json` file.

- `marketingVersion` is the app version
- `buildNumber` is the monotonically increasing build number

Useful commands:

```bash
npm run version:check   # verify tracked version fields match version.json
npm run version:sync    # sync tracked version fields from version.json
npm run version:bump    # bump patch version + build number
```

Release builds are expected to run from already-synced, committed version metadata. The build itself should not rewrite tracked source files.

## Getting Builds

GitHub Actions now builds the app automatically in two ways:

- Every push to `main` creates downloadable workflow artifacts under **GitHub → Actions → (latest run) → Artifacts**
- Every pushed Git tag matching `v*` creates or updates a GitHub Release with the packaged assets attached

Push-build artifacts:

| Artifact | Platform |
|---|---|
| `tasktracker-extreme-3000-windows-portable-exe` | Windows portable EXE |
| `tasktracker-extreme-3000-macos-universal-dmg` | macOS universal DMG |

Release flow:

```bash
git tag v2.1.3
git push origin v2.1.3
```

That tag should match the marketing version in `version.json`.

> **First-run warning:** macOS CI builds are ad-hoc signed, which improves compatibility but does **not** replace Apple notarization. You may still need `System Settings -> Privacy & Security -> Open Anyway`. Windows builds are unsigned, so SmartScreen may still require `More info -> Run anyway`.

## Building From Source

Requires [Rust](https://rustup.rs) and Node.js.

```bash
npm install
npm run version:check
npm run dev
```

For a production build:

```bash
npm run version:check
npm run build
```

## Limitations

- Requires a Desk365 account for ticket integration; the task board and notes still work without it
- Shared-data features depend on the configured sync folder remaining reachable
- Tested primarily on my own machines; your mileage may vary
