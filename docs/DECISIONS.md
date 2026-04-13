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
Status: superseded

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

## 2026-04-11 — Prefer a portable Windows EXE and a universal macOS DMG in CI
Rationale: For this personal-use app, a portable Windows build is more useful than an installer, while macOS distribution is cleaner as a single universal DMG than as split architecture artifacts. This keeps the default downloads simple and platform-appropriate without adding installer complexity.
Status: approved

## 2026-04-11 — Separate push-build artifacts from tagged GitHub releases
Rationale: Building on every push to `main` keeps fast feedback and downloadable artifacts available, while publishing GitHub Releases only from version tags avoids cluttering Releases with every commit. This creates a cleaner release path without losing automatic CI builds.
Status: approved

## 2026-04-12 — Ad-hoc sign macOS CI builds when Apple signing credentials are not configured
Rationale: A macOS `.app` downloaded from the internet is more reliable on Apple Silicon when it is at least ad-hoc signed, even if full Developer ID signing and notarization are not available yet. This improves the default build quality while still being honest that Gatekeeper exceptions may remain necessary.
Status: approved

## 2026-04-12 — On macOS, closing the main window saves and quits instead of hiding to tray
Rationale: The app runs as an accessory on macOS, so hiding the only window makes recovery awkward without a dock icon. Keeping the tray-first hide behavior on Windows preserves the primary-platform workflow while making macOS close semantics practical.
Status: approved

## 2026-04-12 — Treat version bumps on main as release-publish triggers
Rationale: For this repo, increasing the checked-in app version means the work is intentionally ready to publish for now. Releasing directly from `main` when `version.json` changes matches that workflow better than requiring a separate manual tag push.
Status: approved

## 2026-04-13 — Keep Desk365 ticket payloads machine-local while syncing only shared task state
Rationale: Tasks, notes, Desk365 hostname config, and hidden-ticket state benefit from cross-machine sharing, but live Desk365 ticket payloads are fresher and less collision-prone when each machine fetches them directly. This keeps shared storage smaller and avoids turning the sync folder into a ticket cache.
Status: approved

## 2026-04-13 — Use revisioned shared JSON with watcher-driven refresh and conservative merge behavior
Rationale: The shared-folder workflow now needs to tolerate two machines running at once, so the app adds document revisions, file watching, periodic reconciliation, merge-safe task/hidden-ticket saves, and one-time legacy-path import when a new sync folder is chosen. This favors convergence and visible warnings over silent overwrites.
Status: approved

## 2026-04-13 — On first rebuilt launch, import legacy local shared data without auto-adopting old sync paths
Rationale: The first Windows launch after the rebuild should recover old tasks and config automatically when legacy local data exists, but it should do so conservatively by importing into the new local app-data location rather than silently repointing the app at an old synced folder. This reduces surprise and preserves explicit sync-folder choice.
Status: approved

## 2026-04-13 — Use Sortable fallback drag mode and expose a manual legacy import button
Rationale: WebView2 drag behavior on Windows proved less reliable with the native drag path, so the kanban now uses Sortable's fallback drag handling for more predictable cross-column moves. A manual settings button to scan known legacy locations gives the owner an explicit recovery path if first-run import misses older task/config files.
Status: approved

## 2026-04-13 — GitHub Releases should describe commit changes since the previous release
Rationale: Fixed placeholder release notes are too vague to be useful, especially for personal builds that may accumulate several targeted fixes between version bumps. The release workflow now generates the body from commit subjects since the prior release tag while skipping the routine version-bump commit.
Status: approved

## 2026-04-13 — Normalize picked storage paths and allow explicit legacy-file import
Rationale: Windows storage bugs showed that relying only on automatic legacy scanning and raw dialog path strings was too brittle. The app now normalizes dialog-picked filesystem paths, recovers missing sync-folder settings from legacy `local-settings.json`, and lets the owner explicitly import an older task/settings JSON file when needed.
Status: approved
