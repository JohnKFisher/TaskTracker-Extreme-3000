# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- Version source of truth: `version.json`
- Current marketing version: `2.4.0`
- Current build number: `17`

## Overall Status
Working personal-use Tauri desktop app with a revisioned local/shared JSON data model, secure Desk365 credential storage, explicit shared-storage status reporting, and a checked-in deterministic version/build workflow. The app now includes an optional Personal board alongside Work Tasks, configurable column visibility (Standing can be hidden), an on-startup GitHub release check with clickable update banner, auto-capitalization of task titles, polished settings page, a new-ticket shortcut button on the tickets tab, and a refreshed light-mode app icon. All CI/release workflows are operational and have produced multiple successful releases through v2.4.0.

## What Works Now
- Sidebar desktop window with tray behavior, global shortcuts, and a quick-add window
- On macOS, closing the main window saves pending edits and quits instead of hiding
- Kanban task board with drag/drop, inline notes, delete confirmation, and count-aware clear-done confirmation
- Work Tasks board and optional Personal board (same layout, same shared document, visibility toggled per machine)
- Standing column is now optional — can be hidden via Settings > Layout; tasks in Standing are preserved and return if re-enabled
- Collapsed categories auto-expand when a task is added to them (by direct add, move, or sync), except Done which always respects its collapsed state
- Tab counts exclude Done tasks so the count reflects actual remaining work
- Task titles are auto-capitalized on entry (both main board and quick-add window)
- Pin (always-on-top) button is visually distinct: gray/dimmed when off, red when active
- On-startup GitHub release check: if a newer version is published, a persistent clickable banner appears linking to the releases page; silent on network failure
- Notes tab with persisted plain-JSON storage plus conflict-aware save blocking
- Desk365 ticket integration with secure API-key storage and periodic polling
- New ticket "+" button in the tickets bar opens the Desk365 create-ticket page; hidden until domain is configured
- Optional shared sync folder for tasks, notes, Desk365 hostname, and hidden ticket state
- Shared documents carry revision metadata for safer multi-machine reconciliation
- Shared-folder changes refresh via file watching plus 5-minute reconciliation sweep
- Task and hidden-ticket saves merge remote changes instead of blindly overwriting
- Picking a new sync folder can import shared JSON from the current or legacy locations
- First-run legacy import from known pre-Tauri Windows/Electron locations into local app-data
- Settings now has a Try Legacy Import button and an Import From File flow
- Settings page reorganized: Appearance → Layout → Sync Folder → About; Storage status merged into Sync Folder section
- Missing sync-folder settings recovered automatically from legacy local-settings.json when needed
- Windows kanban uses Sortable's fallback drag mode for reliable cross-column dragging
- Tickets keep refresh/reconnect controls at the bottom of the panel
- Hidden-ticket save errors stay in the hidden-ticket flow and do not force a full Desk365 reload
- Desk365 API-key saves verify the key is present in secure storage immediately after writing
- Desk365 fetches are deduplicated and rate-limited to avoid short-window API limits
- Visible warning state when a configured sync folder is unavailable
- Light/Dark/Auto theme toggle in Settings; defaults to Auto, saved per machine
- Hidden tickets are visually distinct when "Show hidden" is active (low opacity + grayscale)
- Auto-compact layout when content overflows the panel; returns to normal when it fits
- Checked-in version/build workflow through version.json and helper scripts
- GitHub Actions push-build workflow produces portable Windows EXE and universal macOS DMG
- GitHub Release workflow publishes assets whenever version.json changes on main, with commit-based release notes
- macOS CI builds use ad-hoc signing
- Refreshed app icon: light-mode kanban with colored column headers and done indicators

## What Is Partial
- Capability narrowing was improved at the desktop capability file level but the app still relies on broad core Tauri access rather than a deeply custom per-command permission model
- Notes conflict handling is conservative: preserves unsaved local draft and blocks overwrite, but no in-app merge UI for conflicting note blobs
- Legacy startup import is one-time and conservative; it does not automatically switch the app onto an old synced folder

## What Is Not Implemented Yet
- A recorded durable known-good rollback anchor in project docs
- Broader automated app-level UI/integration tests beyond version-script checks and Rust unit tests

## Known Limitations And Trust Warnings
- Desk365 integration requires a valid Desk365 account, hostname, and API key
- macOS builds are ad-hoc signed but not notarized; Windows builds are unsigned
- If a configured sync folder goes offline, shared-data features pause until it is reachable again
- Cloud-sync delays can postpone when another machine's file changes arrive, even with file watching and 5-minute reconciliation
- The project is tested primarily on the owner's own machines
- Brief system-theme flash on launch before JS applies a saved non-Auto theme is a known cosmetic limitation

## Setup / Runtime Requirements
- Node.js and npm
- Rust toolchain for local builds and tests
- macOS or Windows desktop environment supported by Tauri v2
- Optional Desk365 credentials for the tickets tab
- GitHub Actions for CI packaging and release publishing

## Important Operational Risks
- Future schema changes to shared JSON files need careful migration handling
- Sync-folder outages are now explicit but still interrupt shared-data workflows until resolved
- Notes remain a single shared blob; simultaneous edits on two machines produce a warning-and-retry workflow
- Credential-store behavior can differ by platform; secure storage changes should be tested on both Windows and macOS

## Recommended Next Priorities
1. Smoke-test the update check banner on a machine running an older build
2. Smoke-test the Windows sync-folder persistence path on a freshly downloaded new build
3. Confirm multi-machine sync behavior on two real machines sharing the same cloud-synced folder
4. Evaluate whether notes should move from a single shared blob to per-section or per-note storage to reduce conflict surface
5. Consider a minor version bump if any further meaningful features land before the next planned release

## Most Recent Durable Known-Good Anchor
v2.4.0 — released 2026-04-17. All CI workflows operational. Icon, settings, update check, Standing column toggle, and new-ticket button all included.
