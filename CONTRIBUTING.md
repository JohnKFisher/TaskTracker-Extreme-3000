# Contributing

This is a personal workflow tool built for my own use. Contributions are welcome but this isn't a product and there's no roadmap or commitment to merge anything.

## What's Welcome

- Bug reports and focused bug fixes
- Documentation improvements
- UI clarity improvements that don't change behavior

## Before Submitting

- Run `npm start` and verify the app launches and the feature you touched still works
- Test the thing you changed manually — there are no automated tests
- Keep pull requests small and focused on one thing

## What To Avoid

- New npm dependencies
- Telemetry, analytics, or any outbound network calls beyond the existing Desk365 API
- Changes to how or where user data is stored without a clear migration path
- Removing or weakening the `data/` gitignore protections — user data and API keys must never be committable
- Background processes or startup entries beyond what's already there

## Feature Requests

Open an issue and describe the use case. I may or may not build it, and I make no timeline commitments.

## Architectural Changes

Open an issue to discuss before writing code. This app is intentionally simple and I'd like to keep it that way.
