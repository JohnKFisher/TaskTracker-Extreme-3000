# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- Version source of truth: `version.json`
- Current marketing version: `2.2.0`
- Current build number: `9`

## Overall Status
Working personal-use Tauri desktop app with a revisioned local/shared JSON data model, secure Desk365 credential storage, explicit shared-storage status reporting, and a checked-in deterministic version/build workflow. Shared task data now watches for cross-machine changes, reconciles periodically, merges common task/hidden-ticket collisions conservatively, and can import legacy data into a newly chosen sync folder. The repo is now at version `2.2.0` / build `9`, with passing local version-script checks and Rust unit tests after the multi-machine sync hardening pass.

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
- Visible warning state when a configured sync folder is unavailable
- Settings tab with storage status plus an About section showing version/build and the public GitHub repo
- Checked-in version/build workflow through `version.json` and helper scripts
- GitHub Actions push-build workflow that produces a portable Windows EXE and a universal macOS DMG
- GitHub Release workflow that publishes the same Windows and macOS assets whenever `version.json` changes on `main`
- macOS CI builds now use ad-hoc signing to improve downloaded-app launch behavior on Apple Silicon

## What Is Partial
- The new GitHub Actions build and release workflows are configured and syntax-checked locally, but their first remote runs still need to complete on GitHub
- Capability narrowing was improved at the desktop capability file level, but the app still relies on core Tauri window/event/webview access rather than a deeply custom per-command permission model
- Notes conflict handling is intentionally conservative: the app preserves the unsaved local draft and blocks overwrite, but it does not yet offer an in-app merge UI for conflicting note blobs

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
- Credential-store behavior can differ by platform, so secure storage changes should continue to be tested on both Windows and macOS

## Recommended Next Priorities
1. Smoke-test the new multi-machine sync behavior on two real machines sharing the same cloud-synced folder.
2. Confirm the `main` push build produces the portable Windows EXE and universal macOS DMG artifacts on GitHub.
3. Record a durable known-good anchor after the shared-sync smoke test and GitHub build artifact checks.

## Most Recent Durable Known-Good Anchor
None recorded yet.
