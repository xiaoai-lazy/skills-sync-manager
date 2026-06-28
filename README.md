# Skills Sync Manager

[中文文档](README.zh.md)

Skills Sync Manager is a desktop app for keeping one main skills library in sync with multiple Claude / agent target directories.

Use it when you want one source of truth for your skills, but need those skills available in several local tool directories without copying files by hand.

## Why this exists

Managing skills across multiple agent directories gets messy quickly:

- Copying skills by hand creates stale duplicates.
- Manual symlinks or junctions are hard to audit later.
- Deleting a skill can leave broken links behind.
- Overwriting an existing target directory can destroy work.

Skills Sync Manager gives you a safer workflow: keep your skills in one main directory, add the target directories you use, and choose which skills should be linked into each target.

## What you can do

- Set one main skills directory as your source library.
- Add multiple target directories for Claude or other local agents.
- See which skills are valid and which ones need fixing.
- Install or uninstall selected skills for the current target.
- Delete a skill from the main library after explicit confirmation.
- Keep settings, targets, and installation records saved locally.

## Download and install

Pre-built installers are available on the [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) page.

Download the package for your platform:

- **Windows**: `.msi` or `.exe`
- **macOS**: `.dmg`
- **Linux**: `.AppImage` or `.deb`

> The installers are currently unsigned. Windows may show a SmartScreen warning, and macOS may require right-clicking the app and choosing Open. This is a code-signing status note; signing will be added in a future release.

## First run

1. Open Skills Sync Manager.
2. Set your main skills directory.
3. Add one or more target directories.
4. Select a target directory from the sidebar.
5. Enable the skills you want to install into that target.

Each skill should be a direct child directory of the main skills directory. A valid skill contains a `SKILL.md` file with YAML frontmatter fields for `name` and `description`.

## Safety model

Skills Sync Manager is intentionally conservative:

- It does not scan your machine for agent directories.
- It does not overwrite existing files or real directories in a target.
- It only uninstalls links that it created and recorded.
- It shows invalid skills but prevents installing them.
- Deleting a main-library skill is irreversible and requires confirmation.

## Link behavior

- **Windows**: uses junctions by default.
- **macOS / Linux**: uses directory symlinks by default.

## Developer notes

Tech stack:

- Tauri 2
- React
- TypeScript
- Vite
- Rust

Run locally:

```bash
npm install
npm run tauri:dev
```

Verify changes:

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## Manual testing

See [docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md](docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md) for the cross-platform verification checklist.
