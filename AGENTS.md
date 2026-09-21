# Project Instructions

This repository inherits the global Codex agreement. Keep this file to durable facts, deliberate exceptions, and pointers Codex cannot reliably infer from source.

## Project facts

- **Purpose / core job:** Personal always-on desktop sidebar for one user's work day — kanban task boards, quick capture, notes, and read-only Desk365 ticket visibility. Single-user, personal-use software; not a product with external customers.
- **Primary platform / stack:** Tauri v2 (Rust backend in `src-tauri/src/main.rs`, single file) with a deliberately framework-free renderer of plain HTML/CSS/JS in `renderer/`. The only vendored dependency is `Sortable.min.js` for drag and drop. Windows is the UX tie-breaker when platform conventions conflict, though the owner's daily driver is macOS.
- **Protected product / architecture invariants:**
  - The renderer stays framework-free and dependency-light. Do not introduce a build step, bundler, or UI framework.
  - Renderer modules are classic scripts loaded in a fixed order (`app.js`, `kanban.js`, `notes.js`, `tickets.js`, `settings.js`) and talk to each other through `window.*` functions. `settings.js` owns per-machine preferences and pushes them outward via `window.apply*`; it loads last, so earlier modules must tolerate its values arriving late.
  - Default window is **380px wide**. Any new control has to survive that width — the tickets bottom bar is already full, which is why the ticket sort toggle sits on the status line instead.
  - Desk365 tickets are read-only in this app and are never written to shared storage; each machine fetches its own.
  - Every per-machine preference needs both a `LocalSettings` field and a case in `normalize_local_settings`. A renderer-side key with no Rust field is silently discarded on save — this has bitten `colorTheme` once already.
  - `LocalSettings` implements `Default` by hand. Keep it in step with the `#[serde(default = ...)]` attributes; the derived version ignores them and produced different values on a fresh install than on an existing one.
- **Unique data / privacy constraints:**
  - The Desk365 API key lives only in the OS credential store, never in `config.json` or any synced file.
  - `local-settings.json` and `window-state.json` are machine-local and must never be synced; `tasks.json`, `notes.json`, `config.json`, and `hidden-tickets.json` are the shared set.
  - The GCS service account key path is machine-local. That key grants bucket write access — treat it like a password.
  - No telemetry, analytics, or network calls beyond Desk365, GCS, and the GitHub release check.
- **Compatibility / deployment constraints:**
  - Shared JSON documents carry `schemaVersion` and `revision`, and two machines can run at once. Migrations must be additive and convergent, and saves merge rather than overwrite.
  - Shared documents do **not** use `deny_unknown_fields`, so an older build that opens a newer document drops fields it does not know and writes them away on the next save. Adding a field to a shared document is therefore a compatibility event: say so, and update every syncing machine before relying on it.
  - When a configured sync folder or bucket is unreachable, the app reports it and pauses shared-data access. Never silently fall back to local storage.
- **Release / distribution constraints:** `version.json` is the single source of truth; `npm run version:sync` propagates it and `version:check` gates `dev`/`build`. `scripts/version-bump.js` only does patch bumps and ignores arguments — it bumps on any invocation, including `--help`. Minor bumps are manual. Pushing a changed `version.json` to `main` publishes a GitHub Release.
- **Recurring quirks / hazards:**
  - Desk365 timestamps arrive in whichever shape the tenant's API returns (`"2026-07-01 09:30:00"` or an ISO string rewritten from an epoch), so they are not reliably parseable. Ticket numbers increase over time and are the safe fallback ordering. Do not mix a date key and a number key inside one comparator — it becomes non-transitive.
  - The Notes "Long-Term" section is still stored under the `generalNotes` key. It was relabelled from "General" in schema v4 without renaming the field so older builds keep reading it.
  - The app uses `tauri-plugin-single-instance`: a second launch exits immediately and focuses the running window, so `npm run dev` appears to do nothing while a release build is open. Quit the running app first.
  - The macOS window can run hidden to the tray, so a launched process may legitimately report zero windows.
  - iCloud `.icloud` placeholder files must stay filtered out of the shared-data watcher.

## Project context

Current explicit project decisions/specifications are authoritative context; historical plans are not instructions to restore old behavior. If docs, decisions, tests, and implementation materially conflict, surface the conflict rather than silently resolving it. Current implementation is authoritative for what the product does today.

When relevant and present, use `docs/DECISIONS.md`, `docs/WHERE_WE_STAND.md`, `docs/WORKING_CHANGELOG.md`, and task-named project specs/docs.

## Verification

- Rust: `cd src-tauri && cargo test`. The toolchain is a rustup install at `~/.cargo/bin` and is not on the default `PATH`.
- Version scripts: `npm run test:version`.
- There are no automated renderer tests. Renderer changes are checked with `node --check` plus a real run, or a static harness that inlines `renderer/styles.css` against the real markup.
