# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- Version source of truth: `version.json`
- Current marketing version: `2.0.0`
- Current build number: `4`

## Overall Status
Working personal-use Tauri desktop app with a local/shared JSON data model, secure Desk365 credential storage, explicit shared-storage status reporting, and a checked-in deterministic version/build workflow. The app is now aligned more closely with the project rules in `AGENTS.md`, especially around secrets, versioning, and visible degraded states.

## What Works Now
- Sidebar desktop window with tray behavior, global shortcuts, and a quick-add window
- Kanban task board with drag/drop, inline notes, delete confirmation, and count-aware clear-done confirmation
- Notes tab with persisted plain-JSON storage
- Desk365 ticket integration using a stored hostname plus secure OS credential storage for the API key
- Optional shared sync folder for tasks, notes, Desk365 settings, and hidden ticket state
- Visible warning state when a configured sync folder is unavailable
- Settings tab with storage status plus an About section showing version/build and the public GitHub repo
- Checked-in version/build workflow through `version.json` and helper scripts
- GitHub Actions packaging workflow with a read-only version check before build

## What Is Partial
- Rust-side verification is only partially validated in this session because the local environment available to Codex does not currently expose `cargo`
- Capability narrowing was improved at the desktop capability file level, but the app still relies on core Tauri window/event/webview access rather than a deeply custom per-command permission model

## What Is Not Implemented Yet
- A recorded durable known-good rollback anchor in project docs
- Broader automated app-level UI/integration tests beyond the version-script checks and Rust unit tests added in source

## Known Limitations And Trust Warnings
- Desk365 integration still depends on a valid Desk365 account, hostname, and API key
- Builds are unsigned, so first-run OS trust prompts are expected on macOS and Windows
- If a configured sync folder goes offline, shared-data features stop until that folder is reachable again by design
- The project is tested primarily on the owner’s own machines

## Setup / Runtime Requirements
- Node.js and npm
- Rust toolchain for local Rust builds and tests
- macOS or Windows desktop environment supported by Tauri
- Optional Desk365 credentials for the tickets tab

## Important Operational Risks
- Any future schema changes to the shared JSON files still need careful migration handling
- Sync-folder outages are now explicit, but they still interrupt shared-data workflows until resolved
- Credential-store behavior can differ by platform, so secure storage changes should continue to be tested on both Windows and macOS

## Recommended Next Priorities
1. Run the new Rust unit tests and a full Tauri build in an environment where `cargo` is available.
2. Add a small integration test layer around shared-storage degraded states and Desk365 connection setup.
3. Record a durable known-good rollback anchor after the aligned build is verified on target machines.

## Most Recent Durable Known-Good Anchor
None recorded yet.
