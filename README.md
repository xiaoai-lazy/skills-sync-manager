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

- **Skill Hub (v0.3)**: browse, install, and update Skills from GitHub / skills.sh repos in the **Skill 中心** view.
- **Smart Paste**: paste a GitHub or skills.sh URL to preview and install a Skill into the main library in one step.
- **Repo management**: add, enable, or disable Skill source repositories from Skill 中心.
- Set one global Skills source directory (configured in Skill 中心, not in the sidebar).
- Add and manage multiple Claude / local Agent target sync directories from the sidebar **目标目录** section.
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
3. (Optional) Open **仓库管理** to add GitHub or skills.sh repos, then use **Discover** to browse and install Skills into the main library. You can also paste a repo or skills.sh link in **Smart Paste** for quick install.
4. In the sidebar **目标目录** section, add the Agent / Claude target directories you want to sync to.
5. Select a target directory from the sidebar to open its detail view.
6. Check and enable the Skills you want to sync; the app will deploy the links automatically.

The sidebar separates **Skill 中心** (main library management) from **目标目录** (per-target sync). The main library path is shown and edited only in Skill 中心, not in the sidebar.

Valid Skill rule: each direct child folder under the main library is treated as one Skill. It must contain a `SKILL.md` file, and the YAML frontmatter must define `name` and `description` fields.

## 6. Safety boundaries

The tool uses a conservative safety model to avoid data risk:

- It does not actively scan or read your whole machine; it only uses directories you add manually.
- It never overwrites real files / folders that already exist in a target directory.
- It only uninstalls links created by this tool and does not modify your native files.
- It automatically blocks invalid Skills from being installed.
- Deleting a Skill from the main library is irreversible and requires a second manual confirmation.

## 7. For developers

Tauri commands are split by responsibility:

- **`install_hub_skill`**: download or import a Skill into the **main library** from Skill Hub (GitHub, skills.sh, Smart Paste). Returns updated Skill Hub local state.
- **`install_skill`**: create a **symlink/junction** from an existing main-library Skill into a **target directory**. Used by target sync, not hub discovery.

Other Skill Hub commands include `discover_skills`, `parse_smart_paste`, `check_skill_updates`, `update_skill`, and `update_all_skills`. See `src/api/skillHub.ts` and `src-tauri/src/commands/skill_hub.rs`.
