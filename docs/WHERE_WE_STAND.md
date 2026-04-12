# WHERE WE STAND

## Project
TaskTracker Extreme 3000

## Current Version / Build
- App version from source-controlled config: `2.0.0`
- Current checked-in UI build stamp: `Built: March 30, 2026, #4`
- Trust warning: the build counter is currently derived from `data/build-number.json`, which is gitignored, so the build number is not yet a deterministic source-controlled release value

## Overall Status
Working personal-use Tauri desktop app with a local task board, notes, optional sync folder support, and Desk365 ticket integration. The repo has working cross-platform packaging automation, but version/build tracking still needs cleanup to fully match the project rules in `AGENTS.md`.

## What Works Now
- Sidebar-style desktop window with custom title bar and tabs
- Kanban task board with multiple columns, drag-and-drop ordering, inline task notes, and done clearing
- Notes tab with persisted local text storage
- Desk365 ticket loading, refresh, polling, hide/unhide state, and open-in-browser actions
- Optional user-selected sync folder for shared task/note/config storage across machines
- System tray integration and global shortcuts for showing the app and quick add
- GitHub Actions workflow for Windows and macOS artifacts

## What Is Partial
- Build numbering exists, but the current counter flow is not source-controlled or reproducible from a clean checkout
- Cross-platform support is implemented, but the repo does not yet explicitly document which platform is primary for product and UX decisions

## What Is Not Implemented Yet
- A source-controlled deterministic build number system that satisfies the project versioning rules
- A recorded durable known-good rollback anchor in project docs
- Repo-local project memory docs were missing before this session and will need to be kept current from here forward

## Known Limitations And Trust Warnings
- Desk365 integration depends on a valid Desk365 account, domain, and API key
- Builds are unsigned, so first-run OS trust prompts are expected on macOS and Windows
- Task, notes, and config data are inspectable JSON files, which is good for recovery, but also means schema discipline matters if the data format changes later
- The build stamp shown in the UI should not yet be treated as authoritative release metadata

## Setup / Runtime Requirements
- Node.js and npm
- Rust toolchain for local builds
- Tauri-compatible desktop environment on macOS or Windows
- Optional Desk365 credentials for the tickets tab

## Important Operational Risks
- Build/version metadata currently does not fully meet the repo’s source-controlled determinism requirement
- Sync-folder behavior is user-chosen and practical, but any future schema or write-path changes will need extra caution to avoid cross-machine data surprises
- Desk365 availability, credentials, or API behavior can affect the tickets tab independently of the local task board

## Recommended Next Priorities
1. Replace the current gitignored build-counter flow with a deterministic source-controlled version/build system.
2. Decide and record the primary target platform for UX tie-breakers.
3. Add a small set of verification steps or tests around the core local data flows and sync-folder path handling.

## Most Recent Durable Known-Good Anchor
None recorded yet.
