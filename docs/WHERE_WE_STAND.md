# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- Version source of truth: `version.json`
- Current marketing version: `2.13.0`
- Current build number: `48`

## Overall Status
Working personal-use Tauri desktop app with a revisioned local/shared JSON data model, secure Desk365 credential storage, explicit shared-storage status reporting, and a checked-in deterministic version/build workflow. Since 2.7.0 it has gained bounded periodic reconciliation that removes deleted and trashed Desk365 tickets from the list, a Newest/A–Z ticket sort toggle, a third Notes section (Short-Term), and fixes for two per-machine preferences that were never actually being persisted.

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
- Task cards are keyboard-navigable: Tab to focus, arrow keys move between cards, Enter expands, Escape collapses, Delete/Backspace triggers delete confirm
- Delete confirm dialog has a third option — Move to Done — which moves the task without tombstoning it
- Cards older than 7 business days (by createdAt) show an amber tint as a staleness indicator (18% color-mix, clearly visible)
- Desk365 ticket cards older than 5 business days (by UpdatedAt) show the same amber stale tint
- Expanding a task card and typing no longer causes the card to close mid-edit when a remote sync fires; in-flight edits are preserved
- Card expand/collapse has a smooth fade+slide animation on open
- Card hover has a more pronounced 3D lift effect with a deeper matching shadow
- Notes tab with three stacked sections — For next meeting, Short-Term, Long-Term — persisted as plain JSON with conflict-aware save blocking
- Notes document schema v4 adds Short-Term; Long-Term keeps the original `generalNotes` key so older builds can still read it
- Desk365 ticket integration with secure API-key storage and periodic polling
- Tickets can be ordered newest-first or alphabetically by subject via a small toggle on the tickets status line; the choice is saved per machine and only re-renders the cached list
- New ticket "+" button in the tickets bar opens the Desk365 create-ticket page; hidden until domain is configured
- Optional shared sync folder for tasks, notes, Desk365 hostname, and hidden ticket state
- Optional GCS (Google Cloud Storage) sync via service account key file + bucket name; takes priority over folder sync when configured
- GCS migration utility copies existing local/sync-folder files to GCS bucket on first setup
- Shared documents carry revision metadata for safer multi-machine reconciliation
- Adaptive reconcile sweep: starts at 60s, grows +10s per quiet cycle, caps at 5 minutes; resets to 60s on any real file-watcher event
- .icloud placeholder files are filtered out before any read attempt to avoid spurious watcher errors
- Task and hidden-ticket saves merge remote changes instead of blindly overwriting
- Picking a new sync folder can import shared JSON from the current or legacy locations
- First-run legacy import from known pre-Tauri Windows/Electron locations into local app-data
- Settings now has a Try Legacy Import button and an Import From File flow
- Settings page reorganized: Appearance → Layout → Sync Folder → GCS → About
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
- GitHub Actions push-build workflow produces a Windows NSIS installer and universal macOS DMG
- GitHub Release workflow publishes assets whenever version.json changes on main, with commit-based release notes
- macOS release builds are wired for Developer ID signing, notarization, and stapling when GitHub secrets are configured
- Refreshed app icon: light-mode kanban with colored column headers and done indicators

## What Is Partial
- Capability narrowing was improved at the desktop capability file level but the app still relies on broad core Tauri access rather than a deeply custom per-command permission model
- Notes conflict handling is conservative: preserves unsaved local draft and blocks overwrite, but no in-app merge UI for conflicting note blobs
- Legacy startup import is one-time and conservative; it does not automatically switch the app onto an old synced folder
- GCS sync is polling-based (no push notifications); worst-case lag is 5 minutes at maximum backoff

## What Is Not Implemented Yet
- A recorded durable known-good rollback anchor in project docs
- Broader automated app-level UI/integration tests beyond version-script checks and Rust unit tests
- Real-time push sync (WebSocket or long-poll) for GCS or folder mode

## Known Limitations And Trust Warnings
- Desk365 integration requires a valid Desk365 account, hostname, and API key
- macOS release builds require Apple Developer signing/notarization secrets; Windows builds are unsigned
- If a configured sync folder goes offline, shared-data features pause until it is reachable again
- GCS sync is polling-only; worst-case change visibility is 5 minutes during quiet periods
- GCS service account key file path is stored in local-settings.json (machine-local, not synced); each machine must configure GCS independently
- Cloud-sync delays can postpone when another machine's file changes arrive, even with file watching and adaptive reconciliation
- Shared JSON documents do not reject unknown fields, so a machine running an older build drops newer fields (such as Short-Term notes) the next time it saves
- The project is tested primarily on the owner's own machines
- Brief system-theme flash on launch before JS applies a saved non-Auto theme is a known cosmetic limitation

## Setup / Runtime Requirements
- Node.js and npm
- Rust toolchain for local builds and tests
- macOS or Windows desktop environment supported by Tauri v2
- Optional Desk365 credentials for the tickets tab
- Optional GCS service account JSON key file and bucket for GCS sync
- GitHub Actions for CI packaging and release publishing

## Important Operational Risks
- Future schema changes to shared JSON files need careful migration handling
- Sync-folder outages are now explicit but still interrupt shared-data workflows until resolved
- Notes remain a single shared blob; simultaneous edits on two machines produce a warning-and-retry workflow
- Credential-store behavior can differ by platform; secure storage changes should be tested on both Windows and macOS
- GCS service account key grants write access to the bucket; protect the key file like a password

## Recommended Next Priorities
1. Hand-check the 2.13.0 renderer changes in the running app: three Notes sections load existing text, typing in Short-Term saves, and the ticket Newest/A–Z toggle reorders and survives a restart
2. Update every syncing machine to 2.13.0 before relying on Short-Term notes — older builds drop the new field when they save
3. Smoke-test GCS sync end-to-end: configure credentials, run migrate, verify tasks appear on a second machine
4. Smoke-test the update check banner on a machine running an older build
5. Smoke-test the Windows sync-folder persistence path on a freshly downloaded new build
6. Confirm multi-machine sync behavior on two real machines sharing the same cloud-synced folder
7. Evaluate whether notes should move from a single shared blob to per-section or per-note storage to reduce conflict surface

## Most Recent Durable Known-Good Anchor
v2.12.0 (build 47) — merged 2026-09-02. Everything in v2.7.0 plus the bounded periodic reconcile that removes deleted and trashed Desk365 tickets from the list. This is the last state verified before the 2.13.0 work described above; 2.13.0 itself has passing Rust and version-script tests but its renderer changes have not yet been exercised by hand in the running app.
