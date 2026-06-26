# Skills Sync Manager

[中文文档](README.zh.md)

A cross-platform desktop app for managing one main skills directory and syncing selected skills into multiple target directories through directory links.

## What the app does

- Manages one configurable main skills directory.
- Validates skills from direct child directories of the main directory.
- A skill is valid when its directory contains `SKILL.md` with YAML frontmatter fields `name` and `description`.
- Shows invalid skill directories with explicit validation errors, but prevents installation.
- Installs valid skills immediately when the user toggles them on for a selected target.
- Uninstalls installed skills immediately when the user toggles them off for a selected target.
- Persists settings, targets, and installation records in a local app-data JSON file.
- Supports direct deletion of a main-directory skill after explicit confirmation and after all recorded links for that skill have been cleaned.

## What the app refuses to do

- The app does not scan the machine for agent directories.
- The app does not overwrite existing target content.
- The app only uninstalls links that it created and recorded.
- Main skill deletion is irreversible in v1 and requires confirmation.

## Tech stack

- Tauri 2
- React
- TypeScript
- Vite
- Rust

## Scripts

- `npm run dev` starts the Vite frontend dev server.
- `npm run build` type-checks and builds the frontend.
- `npm run test` runs frontend tests with Vitest.
- `npm run tauri` runs the Tauri CLI.
- `npm run tauri:dev` starts the Tauri desktop app in development mode.

## Development

```bash
npm install
npm run tauri:dev
```

## Tests

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## Backend checks

Run Rust checks from the Tauri crate directory:

```bash
cd src-tauri
cargo test
```

## Link behavior

- Windows: junction by default.
- macOS/Linux: directory symlink by default.

## Warning

Main skill deletion is irreversible in v1. The app will remove recorded target links before deleting the source skill directory.

## Manual testing

See [docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md](docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md) for the cross-platform verification checklist.
