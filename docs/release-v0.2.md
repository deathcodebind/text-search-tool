# Release v0.2.0

## Summary

This release stabilizes the Pull/Detail flow and prepares the project for a cleaner source control workflow.

## Highlights

- Pull page filter UX refinements and mapping alignment with project docs.
- Task history recording support for pull jobs.
- Detail page route stability improvements.
- Detail page external-browser flow for pages blocked by `X-Frame-Options: SAMEORIGIN`.
- Attachment download path improvements through backend command handling.

## Repository hygiene

- Added root `.gitignore` for:
  - Rust build artifacts (`target/`)
  - frontend build/dependency artifacts (`ui/node_modules/`, `ui/dist/`)
  - local runtime data (`.runtime-data/`, `src-tauri/data/*.db`, `src-tauri/data/session.json`)

## Version bump

- Rust workspace crates: `0.1.0` -> `0.2.0`
- Tauri app version: `0.1.0` -> `0.2.0`
- UI package version: `0.1.0` -> `0.2.0`

## Notes before tagging

- Ensure all intended UI source files under `ui/src/` are added to git.
- Confirm whether deleted tracked files should remain deleted:
  - `src-tauri/data/app.db`
  - `ui/app.js`
- Run final verification:
  - `cargo check -p app`
  - `cd ui && npm run build`

## Suggested tag

- `v0.2.0`
