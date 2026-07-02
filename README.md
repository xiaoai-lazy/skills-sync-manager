# Skills Sync Manager User Guide

[中文文档](README.zh.md)

## 1. Tool overview

Skills Sync Manager is a cross-platform desktop tool for maintaining one central Skills library and batch-linking selected Skills into multiple Claude / local Agent target directories. It replaces the repetitive work of manually copying skill folders.

## 2. Core problems it solves

Maintaining Skills across multiple Agent directories can easily create problems. This tool helps avoid them:

- Manually copied files become stale and inconsistent.
- Manually created symlinks or junctions are hard to track and manage later.
- Deleting a Skill can leave invalid link files behind.
- Manually overwriting files can accidentally damage existing target directory content.

## 3. Core features

- **In-app updates (v0.5)**: check GitHub Releases on startup; install in-app or defer until the next launch.
- **Agent / Project targets (v0.5)**: sidebar **Agent** (global) and **Projects** tree; quick-add for Cursor, Claude Code, Codex, plus custom paths.
- **Skill Hub (v0.4)**: browse, install, and update Skills from GitHub, skills.sh, or self-hosted GitLab repos in the **Skill 中心** view.
- **Self-hosted GitLab**: add private GitLab project URLs as Skill sources; authenticate per site with a Personal Access Token (PAT).
- **Smart Paste**: paste a GitHub, skills.sh, or GitLab URL to preview and install a Skill into the main library in one step.
- **Repo management**: add, enable, or disable Skill source repositories from Skill 中心; manage saved GitLab PATs from **密钥管理**.
- Set one global Skills source directory (configured in Skill 中心, not in the sidebar).
- Add global Agent presets or custom targets under **Agent**; manage projects and project-scoped targets under **Projects** in the sidebar.
- Automatically validate Skills and identify items that need fixing.
- Install / uninstall selected Skill links per target directory.
- Safely delete a Skill from the main library after a second confirmation, reducing accidental deletion risk.
- Persist all settings, directories, and installation records locally.

## 4. Download and install

Go to [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) and download the pre-built installer for your system:

- **Windows**: `.msi` / `.exe`
- **macOS**: `.dmg`
- **Linux**: `.AppImage` / `.deb`

> The installers are signed with [Sigstore](https://www.sigstore.dev/) keyless signing. You can verify them with `cosign`. Windows may still show a SmartScreen warning, and macOS may require right-clicking the app and choosing Open, because this is not native OS code signing.
>
> Verify a downloaded file (example for Windows `.exe`):
>
> ```bash
> cosign verify-blob \
>   --certificate "Skills Sync Manager_x64-setup.exe.crt" \
>   --signature "Skills Sync Manager_x64-setup.exe.sig" \
>   --certificate-identity-regexp '^https://github.com/xiaoai-lazy/skills-sync-manager/\.github/workflows/release\.yml@refs/tags/v.*$' \
>   --certificate-oidc-issuer https://token.actions.githubusercontent.com \
>   "Skills Sync Manager_x64-setup.exe"
> ```
>
> Replace the filenames with the actual installer and matching `.crt`/`.sig` you downloaded.

## 5. Quick start

Complete the basic setup in a few steps:

1. Open the Skills Sync Manager client. The default view is **Skill 中心**.
2. In Skill 中心, set your local Skills main library directory (source directory).
3. (Optional) Open **仓库管理** to add GitHub, skills.sh, or GitLab repos, then use **Discover** to browse and install Skills into the main library. You can also paste a repo or skills.sh / GitLab link in **Smart Paste** for quick install. For private GitLab sites, configure a PAT when prompted (see [GitLab access keys](#gitlab-access-keys) below).
4. Under **Agent** or a **Project**, add target directories (preset chips or custom path).
5. Select a target from the sidebar to open its detail view.
6. Check and enable the Skills you want to sync; the app will deploy the links automatically.

The sidebar has **Skill 中心** (main library), **Agent** (global targets), and **Projects** (project tree). The main library path is shown and edited only in Skill 中心.

Valid Skill rule: each direct child folder under the main library is treated as one Skill. It must contain a `SKILL.md` file, and the YAML frontmatter must define `name` and `description` fields.

### GitLab access keys

Skill 中心 supports self-hosted GitLab private repositories. GitHub and skills.sh public sources work as before; GitLab private projects require a PAT with read access to the project.

1. In **仓库管理**, add a GitLab project URL (HTTPS or SSH-style host/path).
2. When previewing or discovering Skills from a private repo, the app prompts for a **GitLab access key (PAT)** if none is saved for that host.
3. Enter a PAT with at least read access; the app validates it against the GitLab API, then saves it in the OS credential store (Windows Credential Manager, macOS Keychain, or Linux secret service). One PAT per GitLab host is shared by all repos on that site.
4. Open **密钥管理** in **仓库管理** to view configured hosts, update a PAT, or remove a saved key.

PATs are never written to config files or logs. Removing a key deletes it from the system credential store.

## 6. Safety boundaries

The tool uses a conservative safety model to avoid data risk:

- It does not actively scan or read your whole machine; it only uses directories you add manually.
- It never overwrites real files / folders that already exist in a target directory.
- It only uninstalls links created by this tool and does not modify your native files.
- It automatically blocks invalid Skills from being installed.
- Deleting a Skill from the main library is irreversible and requires a second manual confirmation.

## 7. For developers

Tauri commands are split by responsibility:

- **`install_hub_skill`**: download or import a Skill into the **main library** from Skill Hub (GitHub, skills.sh, GitLab, Smart Paste). Returns updated Skill Hub local state.
- **`install_skill`**: create a **symlink/junction** from an existing main-library Skill into a **target directory**. Used by target sync, not hub discovery.

Other Skill Hub commands include `discover_skills`, `parse_smart_paste`, `check_skill_updates`, `update_skill`, and `update_all_skills`. See `src/api/skillHub.ts` and `src-tauri/src/commands/skill_hub.rs`.

### v0.5 config migration

When upgrading from v0.4, the app copies `config.json` to `config.json.backup-v4` in the same directory before migrating. Migration aborts if the backup fails.

### Custom Agent presets

Create `agent-presets.json` in the app data directory to extend or override built-in agents (merged by `id`). See the Chinese README for the JSON schema.

### Release signing (maintainers)

Configure `TAURI_SIGNING_PRIVATE_KEY` and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in GitHub Secrets. The public key lives in `src-tauri/tauri.conf.json`. In-app updater signing coexists with Sigstore installer signing.
