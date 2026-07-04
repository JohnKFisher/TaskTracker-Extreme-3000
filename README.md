# TaskTracker Extreme 3000

Personal desktop task tracker for a small, always-available work sidebar.

TaskTracker Extreme 3000 is a Tauri app for macOS and Windows. It combines a kanban-style task board, quick task capture, notes, optional Desk365 ticket visibility, and optional shared sync through either a folder or Google Cloud Storage.

More Sidelark Labs projects live at [sidelarklabs.com](https://sidelarklabs.com).

## What It Does

- Keeps work and personal tasks in compact kanban boards.
- Provides a quick-add window for fast capture.
- Saves notes alongside tasks.
- Can connect to Desk365 for ticket visibility.
- Stores the Desk365 API key in the OS credential store.
- Can sync shared app data through a chosen folder.
- Can use Google Cloud Storage instead of folder sync.
- Supports light, dark, and system-following appearance.
- Builds packaged macOS and Windows artifacts through GitHub Actions.

## Local Development

Requirements:

- Node.js and npm
- Rust
- macOS or Windows desktop environment supported by Tauri v2

Install dependencies:

```bash
npm install
```

Run the app in development:

```bash
npm run dev
```

Build locally:

```bash
npm run build
```

Run the lightweight checks:

```bash
npm run version:check
npm run test:version
cd src-tauri
cargo check
```

## Versioning

The checked-in version source of truth is `version.json`.

Useful commands:

```bash
npm run version:check
npm run version:sync
npm run version:bump
```

`version:check` verifies tracked version fields match `version.json`.

`version:sync` updates tracked version fields from `version.json`.

`version:bump` bumps the patch version and build number.

For minor or major version changes, edit `version.json` intentionally, then run:

```bash
npm run version:sync
```

## GitHub Builds

The normal build workflow runs on pushes to `main` and on manual dispatch.

It produces:

| Artifact | Platform |
|---|---|
| `TaskTracker Extreme 3000 - Windows Installer` | Windows NSIS installer |
| `TaskTracker Extreme 3000 - macOS Universal DMG` | macOS universal DMG |

The release workflow runs when `version.json` changes on `main` or when manually dispatched. It creates or updates the GitHub Release for the version in `version.json`.

Release assets use human-readable names and publish one Windows installer plus one macOS universal DMG.

## macOS Signing And Notarization

macOS release builds are designed to use Developer ID signing, Apple notarization, and stapling in GitHub Actions.

Add these GitHub repository secrets:

| Secret | Paste This |
|---|---|
| `APPLE_ID` | Your Apple Developer account email address. |
| `APPLE_APP_SPECIFIC_PASSWORD` | An app-specific password from [appleid.apple.com](https://appleid.apple.com/), not your normal Apple ID password. |
| `APPLE_TEAM_ID` | Your 10-character Apple Developer Team ID. |
| `MACOS_CERTIFICATE_P12_BASE64` | Single-line base64 output for the exported Developer ID Application `.p12` certificate. |
| `MACOS_CERTIFICATE_PASSWORD` | The password used when exporting the `.p12` certificate. |
| `MACOS_CODESIGN_IDENTITY` | The exact Developer ID Application identity string shown by `security find-identity -v -p codesigning`. |
| `MACOS_KEYCHAIN_PASSWORD` | A strong temporary password invented for the GitHub Actions keychain. |

Create the base64 certificate value with:

```bash
openssl base64 -A -in /path/to/DeveloperIDApplication.p12
```

The signing identity should look like:

```text
Developer ID Application: Your Name or Company (TEAMID)
```

The release workflow fails the macOS job if required signing secrets are missing. The regular build workflow warns and can still produce an unsigned macOS CI artifact when the secrets are incomplete.

Windows builds are currently unsigned, so Windows may show SmartScreen friction on first launch.

## Sync Notes

By default, app data is local to the machine.

Folder sync can share tasks, notes, Desk365 hostname, and hidden ticket state between machines. If the configured folder is unavailable, shared-data access pauses instead of silently writing divergent local copies.

Google Cloud Storage sync can be used instead of folder sync. When configured, GCS takes priority over folder sync. Treat the GCS service account key file like a password.

## Project Docs

- `docs/WHERE_WE_STAND.md` tracks current project status and known limitations.
- `docs/DECISIONS.md` records durable project decisions.
- `AGENTS.md` contains coding-agent rules for working in this repository.

## License

MIT
