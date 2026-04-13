# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- Version source of truth: `version.json`
- Current marketing version: `2.2.2`
- Current build number: `11`

## Overall Status
Working personal-use Tauri desktop app with a revisioned local/shared JSON data model, secure Desk365 credential storage, explicit shared-storage status reporting, and a checked-in deterministic version/build workflow. Shared task data now watches for cross-machine changes, reconciles periodically, merges common task/hidden-ticket collisions conservatively, can import legacy data into a newly chosen sync folder, and on first rebuilt launch can pull older local Windows/Electron task files into the current app-data location. The current follow-up passes also add manual legacy import controls in Settings, recover missing sync-folder settings from older `local-settings.json` files, normalize dialog-picked storage paths more defensively on Windows, keep hidden-ticket save failures from forcing a full Desk365 reload, and verify secure API-key writes immediately after saving them. The repo is now at version `2.2.2` / build `11`.

## What Works Now
- Sidebar desktop window with tray behavior, global shortcuts, and a quick-add window
- On macOS, closing the main window now saves pending task/note edits and quits instead of hiding the accessory app window
- Kanban task board with drag/drop, inline notes, delete confirmation, and count-aware clear-done confirmation
- Notes tab with persisted plain-JSON storage plus conflict-aware save blocking when another machine changed the same notes blob
- Desk365 ticket integration using a stored hostname plus secure OS credential storage for the API key
- Optional shared sync folder for tasks, notes, Desk365 hostname settings, and hidden ticket state
- Shared task, note, config, and hidden-ticket documents now carry revision metadata for safer multi-machine reconciliation
- Shared-folder changes now refresh via file watching plus a 5-minute reconciliation sweep
- Task and hidden-ticket saves now merge remote changes instead of blindly overwriting stale local copies
- Picking a new sync folder can import shared JSON from the current app data location or known legacy pre-Tauri locations
- First rebuilt launch without a sync folder can import legacy local shared JSON from known pre-Tauri Windows/Electron locations into the current local app-data folder
- Settings now includes a `Try Legacy Import` button to rescan known older storage locations and merge recoverable shared JSON into the current storage target
- Settings now includes an `Import From File…` flow so you can point directly at an older `tasks.json`, `config.json`, `hidden-tickets.json`, `notes.json`, or `local-settings.json`
- Missing sync-folder settings can now be recovered automatically from legacy `local-settings.json` data when the new app-data location does not have them yet
- Windows kanban now uses Sortable's fallback drag mode, stronger full-width headers, and more compact drop zones to make cross-column dragging more reliable and obvious
- Tickets now keep their refresh/reconnect controls at the bottom of the panel to preserve vertical space at the top
- Hidden-ticket save errors now stay in the hidden-ticket flow instead of forcing a full ticket reinitialize/API fetch
- Desk365 API-key saves now verify that the key is still present in secure storage immediately after writing it
- Desk365 fetches are now deduplicated and rate-limited in the renderer so startup/status events do not trip the API's short-window request limit
- Visible warning state when a configured sync folder is unavailable
- Settings tab with storage status plus an About section showing version/build and the public GitHub repo
- Checked-in version/build workflow through `version.json` and helper scripts
- GitHub Actions push-build workflow that produces a portable Windows EXE and a universal macOS DMG
- GitHub Release workflow that publishes the same Windows and macOS assets whenever `version.json` changes on `main`, with release notes generated from commit subjects since the prior release tag
- macOS CI builds now use ad-hoc signing to improve downloaded-app launch behavior on Apple Silicon

## What Is Partial
- The new GitHub Actions build and release workflows are configured and syntax-checked locally, but their first remote runs still need to complete on GitHub
- Capability narrowing was improved at the desktop capability file level, but the app still relies on core Tauri window/event/webview access rather than a deeply custom per-command permission model
- Notes conflict handling is intentionally conservative: the app preserves the unsaved local draft and blocks overwrite, but it does not yet offer an in-app merge UI for conflicting note blobs
- Legacy startup import is one-time and conservative; it does not automatically switch the app onto an old synced folder, though the same scan can now be re-run manually from Settings or by explicitly importing a known older JSON file

## What Is Not Implemented Yet
- A recorded durable known-good rollback anchor in project docs
- Broader automated app-level UI/integration tests beyond the version-script checks and Rust unit tests added in source

## Known Limitations And Trust Warnings
- Desk365 integration still depends on a valid Desk365 account, hostname, and API key
- macOS builds are ad-hoc signed but not notarized, so Privacy & Security approval may still be required; Windows builds remain unsigned
- If a configured sync folder goes offline, shared-data features stop until that folder is reachable again by design
- Cloud-sync delays can still postpone when another machine’s file changes arrive locally, even though the app watches for them and rechecks every 5 minutes
- The project is tested primarily on the owner’s own machines

## Setup / Runtime Requirements
- Node.js and npm
- Rust toolchain for local Rust builds and tests
- macOS or Windows desktop environment supported by Tauri
- Optional Desk365 credentials for the tickets tab
- GitHub Actions for automated CI packaging and tagged release publishing

## Important Operational Risks
- Any future schema changes to the shared JSON files still need careful migration handling
- Sync-folder outages are now explicit, but they still interrupt shared-data workflows until resolved
- Notes remain a single shared blob, so simultaneous editing on two machines still produces a warning-and-retry workflow instead of automatic merging
- If a legacy task file is badly malformed beyond the new tolerant parser, manual cleanup may still be needed
- Credential-store behavior can differ by platform, so secure storage changes should continue to be tested on both Windows and macOS

## Recommended Next Priorities
1. Smoke-test the Windows sync-folder persistence path across launching a freshly downloaded new build, especially when the prior sync-folder setting only exists in legacy local settings.
2. Smoke-test the manual `Import From File…` flow against older `tasks.json` and `local-settings.json` files.
3. Smoke-test hidden ticket hide/unhide without triggering unnecessary ticket API reloads.
4. Smoke-test the multi-machine sync behavior on two real machines sharing the same cloud-synced folder.
5. Confirm the `main` push build produces the portable Windows EXE and universal macOS DMG artifacts on GitHub.

## Most Recent Durable Known-Good Anchor
None recorded yet.
