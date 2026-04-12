# DECISIONS

## 2026-04-11 — Use Tauri v2 for the desktop shell
Rationale: The current app is a cross-platform personal desktop tool and the repo already uses a Tauri v2 structure with a native shell plus web UI. This keeps the app lightweight and avoids Electron-level overhead for a sidebar utility.
Status: approved

## 2026-04-11 — Keep the frontend lightweight and framework-free
Rationale: The renderer is currently plain HTML, CSS, and JavaScript with a vendored drag-and-drop library rather than a heavy frontend framework. That fits the project goal of low friction, inspectable behavior, and minimal dependency churn.
Status: approved

## 2026-04-11 — Store user data locally in JSON, with optional user-chosen sync
Rationale: The current design keeps tasks, notes, config, and hidden-ticket state in local JSON files, and allows an optional sync folder chosen by the user for cross-machine sharing. This supports a local-first workflow while keeping the data inspectable and recoverable.
Status: approved

## 2026-04-11 — Keep machine-specific settings out of synced data
Rationale: The app intentionally keeps `local-settings.json` and window state machine-local while syncing only the user content that benefits from cross-device sharing. That avoids accidental propagation of per-machine UI state.
Status: approved

## 2026-04-11 — Treat Desk365 as an integration, not the product core
Rationale: The codebase and README both position the kanban task board, notes, and local persistence as the core experience, with Desk365 ticket fetching layered on top. This keeps the app useful even when the ticket integration is unavailable or unconfigured.
Status: approved

## 2026-04-11 — Build and distribute through GitHub Actions artifacts
Rationale: The repo already uses a multi-platform GitHub Actions workflow that builds Windows and macOS artifacts on pushes to `main` and manual dispatch. That matches the project’s documented distribution path and avoids making local Rust toolchains a release prerequisite.
Status: approved

## 2026-04-11 — Windows is the primary UX tie-breaker
Rationale: The project still targets both Windows and macOS, but platform-specific UX decisions now resolve in favor of Windows conventions unless there is an explicit exception. This keeps the cross-platform behavior predictable and matches the current product direction.
Status: approved

## 2026-04-11 — Store Desk365 API keys only in the OS credential store
Rationale: `config.json` now stores only non-secret Desk365 settings, while the API key lives in the platform credential store. This removes plaintext secret storage without adding any network dependency or hidden fallback.
Status: approved

## 2026-04-11 — Use version.json as the single checked-in version source
Rationale: App version/build metadata now comes from `version.json`, with helper scripts to sync tracked files and bump patch/build values intentionally. This replaces the old gitignored build-counter flow and makes builds reproducible from committed source.
Status: approved

## 2026-04-11 — Do not silently fall back when a configured sync folder is unavailable
Rationale: If the chosen shared-data folder disappears, the app now reports that condition and pauses shared-data access instead of writing divergent local copies. This protects data integrity and makes degraded states visible.
Status: approved
