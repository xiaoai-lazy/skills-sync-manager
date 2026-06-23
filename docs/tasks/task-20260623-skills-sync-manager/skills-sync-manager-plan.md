# Skills Sync Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform local desktop app that manages one main skills directory and installs selected skills into multiple target directories using Windows junctions or macOS/Linux symlinks.

**Architecture:** Use Tauri for the local desktop shell. React + TypeScript renders the target-centered UI, while Rust owns filesystem operations, config persistence, skill validation, link creation/removal, and safety checks. State is persisted in one app-data JSON file and runtime status is recomputed from the filesystem on refresh.

**Tech Stack:** Tauri 2, React, TypeScript, Vite, Rust, Serde, Vitest, Rust unit/integration tests.

---

## 1. Scope

This implementation builds the first usable MVP for the local Skills Sync Manager. The MVP is intentionally conservative: users explicitly configure one main skills directory and any number of target directories, then install or uninstall skills from the selected target view. The app never scans the machine for agents, never guesses which directories should be managed, and never takes ownership of existing target contents.

### 1.1 In scope

- Implement exactly one configurable main skills directory.
- Implement manually configured target directories with `id`, `name`, and `skillsDir`.
- Validate skills from direct child directories of the main skills directory.
- Treat a skill as valid only when its directory contains `SKILL.md` with YAML frontmatter fields `name` and `description`.
- Show invalid skill directories with explicit validation errors, but prevent installation.
- Install valid skills immediately when the user toggles them on for the selected target.
- Uninstall installed skills immediately when the user toggles them off for the selected target.
- Persist settings, targets, and installation records in a local app-data JSON file.
- Use Windows junctions by default on Windows.
- Use directory symlinks by default on macOS and Linux.
- Refuse to overwrite existing unknown files, real directories, or unknown links in target directories.
- Remove only links that are present in this app's installation records and still validate as links for that record.
- Support direct deletion of a main-directory skill after explicit confirmation and after all recorded links for that skill have been cleaned.

### 1.2 Out of scope

- Multi-main-directory support.
- Team/shared remote skill libraries.
- Automatic discovery of Claude Code, Cursor, project, or custom-agent skill directories.
- Copy-based skill synchronization.
- File-level skill management.
- Batch preview or queued operations.
- Backup, trash, restore, or undo for main skill deletion.
- Automatic repair of externally modified links or deleted records.
- Cloud sync, login, permissions server, or any deployed backend service.

### 1.3 Safety invariants

These rules are non-negotiable and must be enforced in backend code, not only in the UI:

- Installation must not create an installation record until link creation succeeds.
- Uninstallation must not remove an installation record until the recorded link is deleted successfully.
- If a target path already contains same-name unknown content, installation must return a conflict error.
- If an installation record exists but the link path is missing, the state is `missing`; the app must not silently recreate it.
- If an installation record exists but the link path points somewhere else, the state is `mismatch`; the app must not delete it automatically.
- Main skill deletion must abort if any recorded-link cleanup fails.
- Main skill deletion is irreversible in v1, so the UI must require explicit confirmation before calling the backend command.

### 1.4 Definition of done for MVP

The MVP is complete when a user can:

1. Set a main skills directory.
2. See valid and invalid skills from that directory.
3. Add at least two target directories.
4. Install one valid skill into both targets.
5. Modify the source skill and observe the change through the target link.
6. Uninstall the skill from one target without affecting the other target or the source skill.
7. See a conflict when a target already has same-name unknown content.
8. Delete a main skill after confirmation and have all recorded target links cleaned first.
9. Close and reopen the app with settings, targets, and installation records preserved.

## 2. Proposed File Structure

The repository starts as a newly initialized project, so Task 1 will scaffold the app. The implementation should keep Rust backend responsibilities separate from React UI responsibilities. Files that enforce safety rules must live in the Rust backend because UI checks can be bypassed or can drift from filesystem reality.

```text
skills-manager/
├── package.json
├── package-lock.json
├── vite.config.ts
├── tsconfig.json
├── tsconfig.node.json
├── index.html
├── README.md
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles.css
│   ├── api/
│   │   └── commands.ts
│   ├── components/
│   │   ├── Sidebar.tsx
│   │   ├── MainLibraryPanel.tsx
│   │   ├── TargetList.tsx
│   │   ├── TargetDetail.tsx
│   │   ├── SkillRow.tsx
│   │   └── ConfirmDialog.tsx
│   ├── model/
│   │   └── types.ts
│   └── test/
│       └── app.test.tsx
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── commands.rs
│       ├── models.rs
│       ├── config_store.rs
│       ├── skill_library.rs
│       ├── target_registry.rs
│       ├── link_installer.rs
│       ├── skill_remover.rs
│       ├── fs_adapter.rs
│       └── test_support.rs
└── docs/tasks/task-20260623-skills-sync-manager/
    ├── skills-sync-manager-design.md
    └── skills-sync-manager-plan.md
```

### 2.1 Frontend files

- `src/main.tsx`
  - React entry point.
  - Mounts `<App />` into `#root`.

- `src/App.tsx`
  - Owns top-level app state.
  - Loads app state from Tauri on startup.
  - Tracks selected target ID.
  - Passes callbacks to child components.

- `src/styles.css`
  - Contains MVP layout and state-label styling.
  - Avoid component-specific CSS files in v1 unless a component becomes large.

- `src/api/commands.ts`
  - Thin wrapper around Tauri `invoke` calls.
  - Provides typed functions such as `getAppState`, `installSkill`, and `deleteMainSkill`.
  - No business rules here; it only translates frontend calls into backend commands.

- `src/model/types.ts`
  - TypeScript mirror of serialized Rust models.
  - Defines `AppState`, `AppConfig`, `Target`, `Installation`, `SkillView`, `SkillInstallState`, and command payloads.

- `src/components/Sidebar.tsx`
  - Composes `MainLibraryPanel` and `TargetList`.
  - Does not fetch data directly.

- `src/components/MainLibraryPanel.tsx`
  - Shows main skills directory path.
  - Offers set/change directory action.
  - Shows counts for valid and invalid skills.

- `src/components/TargetList.tsx`
  - Renders target records.
  - Supports selecting, adding, editing, and deleting target records through parent callbacks.

- `src/components/TargetDetail.tsx`
  - Renders selected target details and skill rows.
  - Handles empty state when no target is selected.

- `src/components/SkillRow.tsx`
  - Renders one skill in the selected target context.
  - Shows state label and install/uninstall toggle.
  - Disables unsafe actions based on backend-computed state.

- `src/components/ConfirmDialog.tsx`
  - Generic confirmation dialog for destructive actions.
  - Used for main skill deletion and target deletion with recorded installs.

- `src/test/app.test.tsx`
  - Frontend component tests for rendering and command-callback behavior.

### 2.2 Backend files

- `src-tauri/src/main.rs`
  - Tauri application entry point.
  - Registers command handlers.
  - Initializes app state handles if needed.

- `src-tauri/src/commands.rs`
  - Public Tauri command functions.
  - Converts backend domain errors into frontend-safe error strings or error objects.
  - Delegates all business logic to focused modules.

- `src-tauri/src/models.rs`
  - Shared Rust domain models serialized through Serde.
  - Owns stable field names used by the frontend.

- `src-tauri/src/config_store.rs`
  - Loads and saves app JSON config.
  - Creates default config when no config file exists.
  - Writes atomically to reduce corruption risk.

- `src-tauri/src/skill_library.rs`
  - Reads the main skills directory.
  - Validates `SKILL.md` and frontmatter.
  - Returns valid and invalid skill views.

- `src-tauri/src/target_registry.rs`
  - Adds, updates, and deletes target records in config.
  - Validates target paths for install-time use.

- `src-tauri/src/fs_adapter.rs`
  - Encapsulates filesystem operations and platform-specific link behavior.
  - Creates Windows junctions on Windows.
  - Creates directory symlinks on macOS/Linux.
  - Resolves link targets.
  - Deletes links and real skill directories through explicit functions.

- `src-tauri/src/link_installer.rs`
  - Implements install and target-level uninstall.
  - Computes installation state from config plus filesystem reality.
  - Enforces conflict, missing, and mismatch safety rules.

- `src-tauri/src/skill_remover.rs`
  - Implements main-directory skill deletion.
  - Cleans recorded links first.
  - Deletes source skill only after cleanup succeeds.

- `src-tauri/src/test_support.rs`
  - Test-only helpers for temp directories, fixture skills, fixture configs, and fake targets.
  - Use `#[cfg(test)]` so it is unavailable in production builds.

### 2.3 Test placement

Rust tests should live in module-level `#[cfg(test)]` blocks or in backend test support modules instead of a nested `src-tauri/src/tests/` directory. This keeps tests close to the safety logic they verify and avoids ad hoc integration wiring before the crate structure is stable.

Frontend tests can start with one file, `src/test/app.test.tsx`, and split later only if it grows too large.

### 2.4 Config file location

Use the app data directory supplied by Tauri's path API. The logical filename should be:

```text
skills-manager/config.json
```

The exact absolute location is platform-dependent and should not be hard-coded in tests. Backend config-store tests must accept an injected config path so they can run against temporary directories.

## 3. Task Outline

The implementation should proceed backend-first for safety-critical logic, then expose Tauri commands, then wire the frontend. Each task should end with tests or a manual verification command and a small commit.

### Task 1: Scaffold Tauri + React + TypeScript project

**Goal:** Create a runnable empty desktop application with frontend, backend, test commands, and baseline docs.

**Files:**

- Create: `package.json`
- Create: `package-lock.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create or update: `README.md`

**Steps:**

- [ ] Initialize a Vite React TypeScript project in the repository root.
- [ ] Initialize Tauri 2 in `src-tauri/` without adding feature logic.
- [ ] Add npm scripts: `dev`, `build`, `test`, `tauri`, and `tauri:dev`.
- [ ] Add a placeholder React screen titled `Skills Sync Manager`.
- [ ] Register a no-op Tauri window with app name `Skills Sync Manager`.
- [ ] Run `npm install`.
- [ ] Run `npm run build` and fix TypeScript or Vite issues.
- [ ] Run `cd src-tauri && cargo test` and confirm the backend builds.
- [ ] Run `npm run tauri:dev` and confirm the desktop window opens.
- [ ] Commit with message `chore: scaffold tauri react app`.

**Expected result:** A blank but runnable cross-platform Tauri app exists.

### Task 2: Define shared domain models

**Goal:** Create stable Rust and TypeScript models that all later tasks use.

**Files:**

- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/main.rs`
- Create: `src/model/types.ts`

**Rust model requirements:**

- `AppConfig`
  - `version: u32`
  - `settings: Settings`
  - `targets: Vec<Target>`
  - `installations: Vec<Installation>`
- `Settings`
  - `main_skills_dir: Option<PathBuf>` serialized as `mainSkillsDir`
  - `link_strategy: LinkStrategy` serialized as `linkStrategy`
- `LinkStrategy`
  - `Auto`
- `Target`
  - `id: String`
  - `name: String`
  - `skills_dir: PathBuf` serialized as `skillsDir`
  - `created_at: String` serialized as `createdAt`
  - `updated_at: String` serialized as `updatedAt`
- `Installation`
  - `id: String`
  - `skill_dir_name: String` serialized as `skillDirName`
  - `skill_name: String` serialized as `skillName`
  - `source_path: PathBuf` serialized as `sourcePath`
  - `target_id: String` serialized as `targetId`
  - `link_path: PathBuf` serialized as `linkPath`
  - `link_type: LinkType` serialized as `linkType`
  - `created_at: String` serialized as `createdAt`
- `LinkType`
  - `Junction`
  - `Symlink`
- `SkillView`
  - `dir_name: String` serialized as `dirName`
  - `name: Option<String>`
  - `description: Option<String>`
  - `path: PathBuf`
  - `valid: bool`
  - `validation_errors: Vec<String>` serialized as `validationErrors`
- `SkillInstallState`
  - `NotInstalled`
  - `Installed`
  - `Conflict`
  - `Missing`
  - `Mismatch`
  - `SourceMissing`
  - `InvalidSkill`
- `SkillWithTargetState`
  - `skill: SkillView`
  - `state: SkillInstallState`
  - `message: Option<String>`
- `AppState`
  - `config: AppConfig`
  - `skills: Vec<SkillView>`
  - `selected_target_skills: Vec<SkillWithTargetState>` or equivalent target-derived state

**TypeScript model requirements:**

- Mirror serialized field names exactly: `mainSkillsDir`, `linkStrategy`, `skillsDir`, `createdAt`, `updatedAt`, `skillDirName`, `sourcePath`, `targetId`, `linkPath`, `linkType`, `validationErrors`.
- Represent Rust enum serialized values as string union types.

**Steps:**

- [ ] Add Serde dependencies in `src-tauri/Cargo.toml` if not already present.
- [ ] Define Rust models with `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq` where useful.
- [ ] Add `#[serde(rename_all = "camelCase")]` on structs where appropriate.
- [ ] Add explicit enum serialization names in lower camel case, for example `notInstalled`, `installed`, `conflict`.
- [ ] Define matching TypeScript interfaces and union types.
- [ ] Add a small Rust test that serializes an `Installation` and asserts JSON uses camelCase fields.
- [ ] Run `cd src-tauri && cargo test`.
- [ ] Run `npm run build`.
- [ ] Commit with message `feat: define shared domain models`.

**Expected result:** Backend and frontend share a stable contract before commands are implemented.

### Task 3: Implement JSON config store

**Goal:** Persist settings, targets, and installation records in a local JSON config file.

**Files:**

- Create: `src-tauri/src/config_store.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/models.rs` if model adjustments are needed

**Behavior:**

- If the config file does not exist, return default config:
  - `version = 1`
  - `settings.mainSkillsDir = null`
  - `settings.linkStrategy = auto`
  - empty `targets`
  - empty `installations`
- If the config file exists and contains valid JSON, load it.
- If the config file is malformed, return a typed error and do not overwrite it.
- Save should create parent directories.
- Save should be atomic enough for MVP: write to a temporary file in the same directory, then rename over the config file.
- Production path should use the Tauri app-data directory.
- Tests should inject a temporary config path.

**Steps:**

- [ ] Add a `ConfigStore` struct that owns a `config_path: PathBuf`.
- [ ] Add `ConfigStore::new(config_path: PathBuf) -> Self` for tests and command wiring.
- [ ] Add `ConfigStore::load(&self) -> Result<AppConfig, AppError>`.
- [ ] Add `ConfigStore::save(&self, config: &AppConfig) -> Result<(), AppError>`.
- [ ] Add `AppError` or a simple backend error type that can later be converted for Tauri commands.
- [ ] Add unit test: missing config returns default config.
- [ ] Add unit test: valid config round-trips through save/load.
- [ ] Add unit test: malformed JSON returns an error and keeps the file unchanged.
- [ ] Run `cd src-tauri && cargo test config_store`.
- [ ] Commit with message `feat: add json config store`.

**Expected result:** The app can safely persist and reload local state.

### Task 4: Implement skill library validation

**Goal:** Read the main skills directory and classify direct child directories as valid or invalid skills.

**Files:**

- Create: `src-tauri/src/skill_library.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/Cargo.toml`

**Behavior:**

- If `mainSkillsDir` is not configured, return an empty skill list and no filesystem error.
- If the configured main directory does not exist, return a clear error.
- Only inspect direct child directories.
- Ignore regular files in the main directory.
- A child directory is valid only when:
  - `SKILL.md` exists directly inside it.
  - `SKILL.md` starts with YAML frontmatter delimited by `---`.
  - Frontmatter contains non-empty `name`.
  - Frontmatter contains non-empty `description`.
- Invalid child directories should be returned with `valid = false` and explicit `validationErrors`.

**Steps:**

- [ ] Add a frontmatter parser for simple YAML frontmatter.
- [ ] Use `serde_yaml` or a small parser; if using `serde_yaml`, add it to `Cargo.toml`.
- [ ] Implement `list_skills(main_dir: Option<&Path>) -> Result<Vec<SkillView>, AppError>`.
- [ ] Add fixture helper to create skill directories in temporary directories.
- [ ] Add unit test: valid skill with `name` and `description` is returned as valid.
- [ ] Add unit test: missing `SKILL.md` returns invalid skill with `Missing SKILL.md`.
- [ ] Add unit test: missing `name` returns invalid skill with `Missing frontmatter.name`.
- [ ] Add unit test: missing `description` returns invalid skill with `Missing frontmatter.description`.
- [ ] Add unit test: regular files in main directory are ignored.
- [ ] Run `cd src-tauri && cargo test skill_library`.
- [ ] Commit with message `feat: validate skill library`.

**Expected result:** The backend can produce the valid and invalid skill list needed by the UI.

### Task 5: Implement target registry

**Goal:** Manage user-defined targets without touching the target directories themselves.

**Files:**

- Create: `src-tauri/src/target_registry.rs`
- Modify: `src-tauri/src/models.rs` if target payload models are needed

**Behavior:**

- Add target with generated stable ID.
- Update target name and skills directory.
- Delete target configuration.
- Do not delete the target directory from disk.
- If deleting a target with installation records, require an explicit cleanup flow from the command layer or return a structured error indicating recorded installs exist.
- Validate install-time path requirements:
  - path exists
  - path is a directory
  - path is writable or can be tested by creating/removing a small temporary probe file

**Steps:**

- [ ] Add payload models `AddTargetRequest` and `UpdateTargetRequest`.
- [ ] Implement `add_target(config, request) -> Target`.
- [ ] Implement `update_target(config, target_id, request) -> Target`.
- [ ] Implement `delete_target_config(config, target_id) -> Result<(), AppError>` that refuses deletion when records exist unless caller selected cleanup.
- [ ] Implement `validate_target_dir(path: &Path) -> Result<(), AppError>`.
- [ ] Add unit test: add target populates ID and timestamps.
- [ ] Add unit test: update target changes name/path and `updatedAt`.
- [ ] Add unit test: delete target config does not delete the directory.
- [ ] Add unit test: install-time validation fails for missing path.
- [ ] Add unit test: install-time validation fails for regular file path.
- [ ] Run `cd src-tauri && cargo test target_registry`.
- [ ] Commit with message `feat: manage target registry`.

**Expected result:** Targets can be managed independently from install/uninstall behavior.

### Task 6: Implement filesystem adapter

**Goal:** Centralize platform-specific filesystem and link behavior behind a small API.

**Files:**

- Create: `src-tauri/src/fs_adapter.rs`
- Modify: `src-tauri/src/models.rs` if link helper types are needed

**Behavior:**

- `default_link_type()` returns:
  - `LinkType::Junction` on Windows
  - `LinkType::Symlink` on macOS/Linux
- `create_dir_link(source, link_path, link_type)` creates:
  - Windows junction with `std::os::windows::fs::symlink_dir` only if symlink is explicitly used; for junction use a junction-capable crate or Windows API wrapper.
  - Unix directory symlink with `std::os::unix::fs::symlink`.
- `path_exists(path)` returns whether any filesystem object exists at the path.
- `is_dir(path)` returns whether path is a directory.
- `link_target(path)` resolves a link target when possible.
- `remove_recorded_link(path, expected_target)` deletes only when the path is a link and resolves to the expected target.
- `delete_real_dir(path)` recursively deletes a real directory only for main skill deletion.

**Implementation note:** For Windows junction support, use a well-maintained crate such as `junction` if direct standard-library behavior is insufficient. The implementation must be tested on Windows before release.

**Steps:**

- [ ] Implement `default_link_type` using conditional compilation.
- [ ] Implement Unix symlink creation behind `#[cfg(unix)]`.
- [ ] Implement Windows junction creation behind `#[cfg(windows)]`.
- [ ] Implement link target resolution.
- [ ] Implement safe recorded-link removal.
- [ ] Add unit test: default link type matches current OS.
- [ ] Add unit test: unknown real directory is not removed by recorded-link removal.
- [ ] Add unit test: unknown regular file is not removed by recorded-link removal.
- [ ] Add platform integration test: create link, resolve target, remove link.
- [ ] Run `cd src-tauri && cargo test fs_adapter`.
- [ ] Commit with message `feat: add filesystem adapter`.

**Expected result:** All filesystem operations used by install/uninstall are isolated and testable.

### Task 7: Implement link installer

**Goal:** Install valid source skills into selected target directories and compute target-specific status.

**Files:**

- Create: `src-tauri/src/link_installer.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/config_store.rs` if helper save/update APIs are needed

**Behavior:**

- Installation inputs:
  - current config
  - selected target ID
  - skill directory name
  - current skill library list
- Find the target by ID.
- Find the skill by directory name.
- Reject invalid skill.
- Validate target directory.
- Compute `linkPath = target.skillsDir / skill.dirName`.
- If `linkPath` exists and is already the recorded correct link, return installed state.
- If `linkPath` exists and is not a recorded correct link, return conflict error.
- If `linkPath` does not exist, create link and append installation record.
- Use stable record fields: source path, target ID, link path, link type, timestamps.

**Steps:**

- [ ] Implement `install_skill(config, target_id, skill_dir_name, skills) -> Result<AppConfig, AppError>`.
- [ ] Add helper `find_installation(config, target_id, skill_dir_name)`.
- [ ] Add helper `compute_skill_state(config, target, skill)`.
- [ ] Add helper `compute_target_skill_states(config, target_id, skills)`.
- [ ] Add unit test: installing valid skill creates link and installation record.
- [ ] Add unit test: installing invalid skill fails.
- [ ] Add unit test: target same-name real directory returns conflict.
- [ ] Add unit test: target same-name regular file returns conflict.
- [ ] Add unit test: repeated install of existing recorded correct link is idempotent.
- [ ] Run `cd src-tauri && cargo test link_installer`.
- [ ] Commit with message `feat: install skills with recorded links`.

**Expected result:** The backend can safely install skills and report install status.

### Task 8: Implement target-level uninstall

**Goal:** Remove installed skills from a target without touching source skills or unknown target content.

**Files:**

- Modify: `src-tauri/src/link_installer.rs`
- Modify: `src-tauri/src/fs_adapter.rs` if more safe-delete helpers are needed

**Behavior:**

- Uninstall inputs:
  - current config
  - target ID
  - skill directory name
- Find installation record by target ID and skill directory name.
- If no record exists, return a clear error or no-op result according to command UX; prefer clear error for MVP.
- Validate `linkPath` exists.
- Validate `linkPath` is a link and resolves to `sourcePath`.
- Remove link.
- Remove installation record after successful link deletion.
- If validation or deletion fails, preserve the record and return an error.

**Steps:**

- [ ] Implement `uninstall_skill(config, target_id, skill_dir_name) -> Result<AppConfig, AppError>`.
- [ ] Add unit test: uninstall removes recorded link and record.
- [ ] Add unit test: uninstall does not delete source skill.
- [ ] Add unit test: uninstall does not delete unknown real directory at link path.
- [ ] Add unit test: missing link preserves record and returns `missing` style error.
- [ ] Add unit test: mismatched link preserves record and returns `mismatch` style error.
- [ ] Run `cd src-tauri && cargo test link_installer`.
- [ ] Commit with message `feat: uninstall recorded skill links safely`.

**Expected result:** Target-level uninstall enforces the app's core safety rule.

### Task 9: Implement main skill deletion

**Goal:** Delete a source skill directly, but only after all recorded links for that skill have been cleaned.

**Files:**

- Create: `src-tauri/src/skill_remover.rs`
- Modify: `src-tauri/src/models.rs` if delete result types are needed

**Behavior:**

- Delete inputs:
  - current config
  - skill directory name
  - explicit confirmation boolean or command-layer confirmation token
- Reject if confirmation is false.
- Find source skill path from main skills directory and skill directory name.
- Find all installation records with the same `skillDirName` or `sourcePath`.
- Remove each recorded link using safe recorded-link deletion.
- If any link cleanup fails, abort and keep the source skill.
- If all link cleanup succeeds, delete the source skill directory recursively.
- Remove related installation records only after source deletion succeeds.

**Steps:**

- [ ] Implement `delete_main_skill(config, skill_dir_name, confirmed) -> Result<AppConfig, AppError>`.
- [ ] Add a delete result model containing removed link count if useful for UI.
- [ ] Add unit test: rejects when confirmation is false.
- [ ] Add unit test: deletes uninstalled source skill.
- [ ] Add unit test: cleans multiple recorded links before deleting source skill.
- [ ] Add unit test: aborts source deletion if one recorded link cleanup fails.
- [ ] Add unit test: successful deletion removes related installation records.
- [ ] Run `cd src-tauri && cargo test skill_remover`.
- [ ] Commit with message `feat: delete main skills after link cleanup`.

**Expected result:** Destructive source deletion is implemented with backend-enforced ordering.

### Task 10: Expose Tauri commands

**Goal:** Provide a typed command boundary between React and Rust.

**Files:**

- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/api/commands.ts`

**Commands:**

- `get_app_state(selectedTargetId?: string)`
- `set_main_skills_dir(path: string)`
- `list_skills()`
- `add_target(name: string, skillsDir: string)`
- `update_target(targetId: string, name: string, skillsDir: string)`
- `delete_target(targetId: string, cleanupRecordedLinks: boolean)`
- `install_skill(targetId: string, skillDirName: string)`
- `uninstall_skill(targetId: string, skillDirName: string)`
- `delete_main_skill(skillDirName: string, confirmed: boolean)`
- `open_path(path: string)` if straightforward with Tauri opener APIs; otherwise defer and remove from UI.

**Behavior:**

- Commands load config, perform operation, save config if mutated, then return fresh app state.
- Commands must not expose raw Rust panic messages.
- Errors should include user-safe messages such as conflict path, missing path, or mismatch reason.

**Steps:**

- [ ] Register all MVP commands in `main.rs`.
- [ ] Implement command wrappers in `commands.rs`.
- [ ] Add frontend wrappers in `src/api/commands.ts` using Tauri `invoke`.
- [ ] Ensure frontend wrapper names use TypeScript camelCase.
- [ ] Add command-level tests for non-Tauri pure functions where possible.
- [ ] Run `cd src-tauri && cargo test`.
- [ ] Run `npm run build`.
- [ ] Commit with message `feat: expose tauri commands`.

**Expected result:** The frontend can call backend operations without knowing filesystem details.

### Task 11: Build frontend layout

**Goal:** Implement the target-centered MVP UI.

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Create: `src/components/Sidebar.tsx`
- Create: `src/components/MainLibraryPanel.tsx`
- Create: `src/components/TargetList.tsx`
- Create: `src/components/TargetDetail.tsx`
- Create: `src/components/SkillRow.tsx`

**Behavior:**

- App loads state on startup.
- Left sidebar has a main-directory section and target-directory section.
- Main-directory section shows current path or empty setup state.
- Target list shows all configured targets and selected target.
- Main content shows selected target details.
- If no target is selected, show an empty state prompting the user to add/select a target.
- Skill rows show name, description, directory name, status label, and toggle.
- Invalid skills are visible in a separate section or visually marked as invalid, but not installable.

**Steps:**

- [ ] Implement `App` state: `appState`, `selectedTargetId`, `loading`, `error`.
- [ ] Implement state refresh helper that calls `getAppState(selectedTargetId)`.
- [ ] Implement sidebar composition.
- [ ] Implement target selection.
- [ ] Implement skill list rendering for selected target.
- [ ] Implement invalid skill rendering.
- [ ] Add CSS for two-column layout, status badges, disabled rows, and error banners.
- [ ] Run `npm run build`.
- [ ] Commit with message `feat: build target-centered ui layout`.

**Expected result:** The app can display backend state clearly even before mutation controls are complete.

### Task 12: Implement immediate install/uninstall interaction

**Goal:** Wire skill toggles to backend install and uninstall commands.

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/components/TargetDetail.tsx`
- Modify: `src/components/SkillRow.tsx`
- Modify: `src/api/commands.ts` if needed

**Behavior:**

- Toggle on for `notInstalled` calls `installSkill` immediately.
- Toggle off for `installed` calls `uninstallSkill` immediately.
- During the command, disable that row to prevent duplicate operations.
- After success, refresh app state.
- After failure, show error and refresh app state so filesystem-derived status is accurate.
- Disable toggles for `conflict`, `missing`, `mismatch`, `sourceMissing`, and `invalidSkill`.

**Steps:**

- [ ] Add `onToggleSkill(targetId, skillDirName, currentState)` callback in `App`.
- [ ] Pass callback to `TargetDetail` and `SkillRow`.
- [ ] Implement per-row pending state.
- [ ] Call install/uninstall based on current backend state.
- [ ] Display backend errors in a visible banner.
- [ ] Refresh state in `finally` after mutation.
- [ ] Run `npm run build`.
- [ ] Commit with message `feat: wire immediate skill toggles`.

**Expected result:** User toggles cause immediate safe backend operations.

### Task 13: Implement dangerous operation confirmation

**Goal:** Add explicit confirmation before irreversible main skill deletion.

**Files:**

- Create: `src/components/ConfirmDialog.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/MainLibraryPanel.tsx` or `TargetDetail.tsx` depending where delete action is placed
- Modify: `src/styles.css`

**Behavior:**

- Delete action is available only for valid source skills.
- Clicking delete opens confirmation dialog.
- Dialog states that deletion is irreversible in v1.
- Dialog shows skill directory name and, when available, affected recorded link count.
- Confirm calls `deleteMainSkill(skillDirName, true)`.
- Cancel closes dialog without backend call.
- After success or failure, refresh state.

**Steps:**

- [ ] Implement reusable `ConfirmDialog` with title, message, confirm label, cancel label.
- [ ] Add selected-delete-skill state to `App`.
- [ ] Wire delete button from skill row or source skill list.
- [ ] Show irreversible warning text.
- [ ] Call backend only on confirm.
- [ ] Refresh app state after command completes.
- [ ] Run `npm run build`.
- [ ] Commit with message `feat: confirm main skill deletion`.

**Expected result:** Main skill deletion cannot happen through a single accidental click.

### Task 14: Add frontend tests

**Goal:** Verify UI rendering and command wiring without relying on real filesystem operations.

**Files:**

- Create: `src/test/app.test.tsx`
- Modify: `package.json`
- Modify: `vite.config.ts` or `vitest.config.ts` if needed

**Test cases:**

- Renders app title and main directory section.
- Renders target list from mocked app state.
- Selecting a target shows its skill rows.
- A `notInstalled` skill toggle calls install command.
- An `installed` skill toggle calls uninstall command.
- Conflict/missing/mismatch states render disabled controls.
- Delete skill button opens confirmation dialog.
- Canceling confirmation does not call delete command.
- Confirming deletion calls delete command with `confirmed = true`.

**Steps:**

- [ ] Add Vitest and React Testing Library dependencies if not already present.
- [ ] Mock `src/api/commands.ts` in tests.
- [ ] Add test fixtures for app state, targets, valid skills, invalid skills, and conflict states.
- [ ] Write rendering tests.
- [ ] Write interaction tests.
- [ ] Run `npm run test`.
- [ ] Commit with message `test: cover frontend interactions`.

**Expected result:** Frontend behavior is covered independently from Tauri runtime.

### Task 15: Add backend integration tests

**Goal:** Verify core safety behavior with temporary directories.

**Files:**

- Modify: `src-tauri/src/config_store.rs`
- Modify: `src-tauri/src/skill_library.rs`
- Modify: `src-tauri/src/fs_adapter.rs`
- Modify: `src-tauri/src/link_installer.rs`
- Modify: `src-tauri/src/skill_remover.rs`
- Create or modify: `src-tauri/src/test_support.rs`

**Test cases:**

- Config default/load/save/malformed behavior.
- Skill validation for valid, missing file, missing name, missing description.
- Install creates a link and an installation record.
- Install blocks same-name real directory.
- Install blocks same-name regular file.
- Install blocks unknown same-name link.
- Uninstall removes only the recorded link.
- Uninstall preserves source skill.
- Uninstall refuses missing or mismatched link and preserves record.
- Main skill deletion cleans multiple recorded links.
- Main skill deletion aborts when recorded-link cleanup fails.
- Main skill deletion removes related installation records after success.

**Steps:**

- [ ] Add tempdir-based helpers to create valid skill fixtures.
- [ ] Add tempdir-based helpers to create target directories.
- [ ] Add helper to build config with one main dir and one or more targets.
- [ ] Add tests for all safety cases above.
- [ ] Run `cd src-tauri && cargo test`.
- [ ] Commit with message `test: cover backend safety flows`.

**Expected result:** The safety invariants are enforced by tests close to backend code.

### Task 16: Add manual cross-platform verification checklist

**Goal:** Document the exact manual checks needed on Windows, macOS, and Linux.

**Files:**

- Create: `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md` or update it if already created
- Modify: `README.md` if useful

**Checklist content:**

- Windows:
  - Install a valid skill and confirm target entry is a junction.
  - Modify `SKILL.md` in source and confirm target sees the change.
  - Uninstall and confirm source is untouched.
  - Create same-name real directory in target and confirm install is blocked.
- macOS:
  - Install a valid skill and confirm target entry is a symlink.
  - Modify source and confirm target sees the change.
  - Uninstall and confirm source is untouched.
  - Create same-name unknown link and confirm install is blocked.
- Linux:
  - Same as macOS symlink behavior.
- All platforms:
  - Close/reopen app and confirm config persists.
  - Delete main skill after confirmation and confirm recorded links are removed first.

**Steps:**

- [ ] Create test document with platform checklist.
- [ ] Reference the checklist from README.
- [ ] Commit with message `docs: add cross-platform verification checklist`.

**Expected result:** Release verification does not depend on memory or ad hoc testing.

### Task 17: Polish error messaging and state labels

**Goal:** Make safety refusals understandable to non-implementers.

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/models.rs` if structured errors are added
- Modify: `src/components/SkillRow.tsx`
- Modify: `src/styles.css`

**User-facing labels:**

- `notInstalled` -> `未安装`
- `installed` -> `已安装`
- `conflict` -> `冲突`
- `missing` -> `缺失`
- `mismatch` -> `异常`
- `sourceMissing` -> `源缺失`
- `invalidSkill` -> `无效 skill`

**Error message requirements:**

- Conflict message includes the target path that already exists.
- Missing message explains that the app record exists but the link is gone.
- Mismatch message explains that the path no longer points to the recorded source.
- Delete failure message explains that source skill was not deleted because link cleanup failed.
- Invalid skill message includes validation errors from `SkillView.validationErrors`.

**Steps:**

- [ ] Normalize backend error messages.
- [ ] Map backend states to Chinese labels in one frontend helper.
- [ ] Add status badge styling.
- [ ] Add inline explanatory text for unsafe disabled toggles.
- [ ] Run `npm run test`.
- [ ] Run `cd src-tauri && cargo test`.
- [ ] Commit with message `feat: polish safety state messaging`.

**Expected result:** Users can understand why the app refused to perform an unsafe action.

### Task 18: Update docs and final validation

**Goal:** Bring docs, tests, and build into a release-ready MVP state.

**Files:**

- Modify: `README.md`
- Modify: `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-plan.md`
- Modify: `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md` if created
- Optionally create: `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-release-checklist.md`

**README content:**

- What the app does.
- What the app refuses to do.
- Development commands.
- Test commands.
- Cross-platform link behavior.
- Warning that main skill deletion is irreversible in v1.

**Final checks:**

- `npm run test`
- `npm run build`
- `cd src-tauri && cargo test`
- `npm run tauri:dev`
- Manual smoke test with temporary directories.

**Steps:**

- [ ] Update README.
- [ ] Update plan checkbox progress if implementation is being executed from this plan.
- [ ] Create release checklist if packaging or distribution work starts.
- [ ] Run all validation commands.
- [ ] Fix any failures before claiming completion.
- [ ] Commit with message `docs: finalize skills manager mvp docs`.

**Expected result:** The repository contains a tested MVP and clear usage/development documentation.

## 4. Validation Commands

Run validation frequently. Do not wait until the end of the full implementation.

### 4.1 Initial setup validation

```bash
npm install
npm run build
cd src-tauri && cargo test
```

Expected:

- `npm install` completes and creates `package-lock.json`.
- `npm run build` completes with no TypeScript errors.
- `cargo test` completes with all backend tests passing.

### 4.2 Frontend validation

```bash
npm run test
npm run build
```

Expected:

- `npm run test` passes all Vitest tests.
- `npm run build` passes TypeScript and Vite production build checks.

### 4.3 Backend validation

```bash
cd src-tauri && cargo test
```

Expected:

- Config-store tests pass.
- Skill-library tests pass.
- Filesystem-adapter tests pass on the current OS.
- Link-installer safety tests pass.
- Skill-remover safety tests pass.

### 4.4 Desktop runtime validation

```bash
npm run tauri:dev
```

Expected:

- Desktop window opens.
- App title is visible.
- User can set or display a main skills directory.
- User can add a target directory.
- User can install and uninstall a valid skill using temporary test directories.

### 4.5 Manual fixture setup

Use temporary directories for manual checks. Do not use real Claude or project skill directories while testing destructive flows.

Example structure:

```text
manual-fixtures/
├── main-skills/
│   ├── valid-skill/
│   │   └── SKILL.md
│   └── invalid-skill/
│       └── README.md
├── target-a/
└── target-b/
```

Example valid `SKILL.md`:

```markdown
---
name: valid-skill
description: A valid test skill.
---

# Valid Skill
```

Checks:

- Set `manual-fixtures/main-skills` as the main directory.
- Confirm `valid-skill` is valid.
- Confirm `invalid-skill` is invalid.
- Add `target-a` and `target-b`.
- Install `valid-skill` into both targets.
- Edit source `SKILL.md`; confirm both targets reflect the change.
- Uninstall from `target-a`; confirm source and `target-b` remain intact.
- Create `target-a/valid-skill` as a real directory; confirm reinstall is blocked as conflict.

### 4.6 Final validation before completion

Run all of these before claiming implementation complete:

```bash
npm run test
npm run build
cd src-tauri && cargo test
cd .. && npm run tauri:dev
```

Completion cannot be claimed if any command fails. If a command is skipped because a dependency is unavailable on the current machine, document the skipped command and reason in the final handoff.

## 5. Open Implementation Decisions

The design is approved, but a few implementation details should be fixed during Task 1 instead of remaining vague during development.

### 5.1 Tauri initialization

Decision: use the current Tauri 2 scaffold and keep the generated structure unless it conflicts with the file responsibilities in this plan.

Recommended initialization path:

```bash
npm create tauri-app@latest .
```

When prompted, choose:

- Frontend: React
- Language: TypeScript
- Package manager: npm

If the scaffold command cannot run in a non-empty directory, initialize in a temporary directory and copy the generated project files into this repository without overwriting `docs/tasks/`.

### 5.2 Config filename and app identity

Decision:

- App display name: `Skills Sync Manager`
- App identifier: `com.skillsmanager.app` or a more specific reverse-domain identifier if the project later gets one
- Logical config filename: `config.json`
- Logical config folder under app data: `skills-manager`

The backend should not hard-code platform-specific absolute app-data paths. It should ask Tauri for app data location in command wiring and inject concrete paths into `ConfigStore`.

### 5.3 Frontend test tooling

Decision: use Vitest plus React Testing Library.

Required dev dependencies:

- `vitest`
- `@testing-library/react`
- `@testing-library/user-event`
- `@testing-library/jest-dom`
- `jsdom`

### 5.4 Rust test tooling

Decision: use temporary directories for all filesystem tests.

Recommended dev dependency:

- `tempfile`

Optional dependency:

- `junction` or equivalent Windows junction crate if direct Windows API implementation is not chosen.

### 5.5 `open_path` command

Decision: include `open_path` only if Tauri opener support is straightforward in Task 10. If it causes packaging or permission friction, remove the button and defer the command. This command is convenience-only and must not block the MVP.

### 5.6 Timestamp and ID generation

Decision: use simple generated IDs and ISO-like timestamps. They only need to be stable and unique enough for local config records.

Recommended approach:

- Rust IDs: prefix plus timestamp/counter or UUID if a small dependency is acceptable.
- Timestamps: UTC RFC3339 strings if using a time crate; otherwise generated in a small helper consistently.

If adding dependencies, prefer clarity over avoiding tiny utilities because config records are user-visible troubleshooting data.

### 5.7 State mismatch handling

Decision: v1 should detect mismatches but not repair them automatically.

The only allowed user action for abnormal states in v1 is to show an explanation. A future version may add explicit `remove record` or `repair link` actions, but those are not part of this MVP.

## 6. Execution Notes

### 6.1 Recommended execution mode

Use `subagent-driven-development` for implementation. Dispatch one fresh implementation subagent per task, then review the diff before moving to the next task. This project has clear task boundaries and safety-critical backend behavior, so isolated task execution plus review is safer than a long inline coding session.

If implementing inline, use `executing-plans` and keep commits small. Do not batch more than two tasks before running tests.

### 6.2 Backend-first ordering

The task order is intentional:

1. Scaffold the app.
2. Define models.
3. Build config persistence.
4. Build skill validation.
5. Build target registry.
6. Build filesystem adapter.
7. Build install/uninstall/delete safety flows.
8. Expose commands.
9. Build UI.

Do not implement destructive UI actions before backend safety checks exist. The UI should never be the only layer preventing accidental deletion.

### 6.3 Test data rule

All automated tests must use temporary directories. Manual tests should also use temporary directories until final smoke testing. Never run development tests against:

- `C:/Users/<user>/.claude/skills`
- `~/.claude/skills`
- real project-level skill directories
- real agent skill directories

This prevents accidental deletion during development of link cleanup and main skill deletion.

### 6.4 Commit discipline

Commit after each task when validation passes. Suggested pattern:

```bash
git status --short
git add <changed-files>
git commit -m "<type>: <short task summary>" -m "Co-Authored-By: Claude <noreply@anthropic.com>"
```

Use these types:

- `chore` for scaffolding and tooling
- `feat` for implementation behavior
- `test` for test coverage
- `docs` for documentation-only changes

### 6.5 Error handling discipline

Backend errors should be precise and user-safe. Do not return raw panics or low-level debug dumps to the frontend. Include enough information for the user to fix the issue:

- conflict path
- missing link path
- expected source path for mismatch
- target path that is not writable
- invalid skill validation error

### 6.6 Security and safety discipline

This is local filesystem software. Treat every delete as dangerous.

Implementation must preserve these constraints:

- Do not follow a record blindly; always re-check the filesystem before deleting.
- Do not delete unknown same-name content.
- Do not delete source skills through target uninstall.
- Do not delete target directories when deleting target config.
- Do not auto-repair missing/mismatched records in v1.
- Do not add broad recursive scans.

### 6.7 Cross-platform discipline

Do not assume Windows path separators in code or tests. Use `PathBuf` and path joins in Rust, and avoid frontend string parsing for path logic. Frontend may display paths as strings, but backend must own path interpretation.

Platform-specific link creation must be isolated in `fs_adapter.rs`. Other modules should ask for `default_link_type()` and `create_dir_link()` instead of branching on OS.

### 6.8 Plan self-review checklist

Before executing this plan, confirm:

- Every design requirement has a corresponding task.
- Backend safety rules are implemented before UI destructive actions.
- Tests cover valid, invalid, conflict, missing, mismatch, install, uninstall, and main deletion flows.
- No task requires real user skill directories.
- The plan path follows `oxygen-standard-docs`: `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-plan.md`.

### 6.9 Execution handoff

Plan complete and saved to `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-plan.md`.

Two execution options:

1. **Subagent-Driven (recommended)** - Dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints.

Recommended choice: **Subagent-Driven**, because the backend contains safety-critical filesystem behavior and each task has clear boundaries.

---

## 7. Detailed backend contracts

This section locks down the backend API shape before implementation. Engineers may adjust syntax to match Rust compiler feedback, but they should preserve these responsibilities, names, and safety semantics unless a later review explicitly changes the plan.

### 7.1 Rust domain model sketch

Create `src-tauri/src/models.rs` with this shape:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: u32,
    pub settings: Settings,
    pub targets: Vec<Target>,
    pub installations: Vec<Installation>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            settings: Settings::default(),
            targets: Vec::new(),
            installations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub main_skills_dir: Option<PathBuf>,
    pub link_strategy: LinkStrategy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            main_skills_dir: None,
            link_strategy: LinkStrategy::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkStrategy {
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub id: String,
    pub name: String,
    pub skills_dir: PathBuf,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub skill_dir_name: String,
    pub skill_name: String,
    pub source_path: PathBuf,
    pub target_id: String,
    pub link_path: PathBuf,
    pub link_type: LinkType,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkType {
    Junction,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    pub dir_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: PathBuf,
    pub valid: bool,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SkillInstallState {
    NotInstalled,
    Installed,
    Conflict,
    Missing,
    Mismatch,
    SourceMissing,
    InvalidSkill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillWithTargetState {
    pub skill: SkillView,
    pub state: SkillInstallState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub config: AppConfig,
    pub skills: Vec<SkillView>,
    pub selected_target_id: Option<String>,
    pub selected_target_skills: Vec<SkillWithTargetState>,
}
```

Add this serialization test in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_serializes_with_camel_case_fields() {
        let installation = Installation {
            id: "inst_1".to_string(),
            skill_dir_name: "brainstorming".to_string(),
            skill_name: "brainstorming".to_string(),
            source_path: PathBuf::from("/main/brainstorming"),
            target_id: "target_1".to_string(),
            link_path: PathBuf::from("/target/brainstorming"),
            link_type: LinkType::Symlink,
            created_at: "2026-06-23T00:00:00Z".to_string(),
        };

        let value = serde_json::to_value(installation).expect("serialize installation");

        assert!(value.get("skillDirName").is_some());
        assert!(value.get("skillName").is_some());
        assert!(value.get("sourcePath").is_some());
        assert!(value.get("targetId").is_some());
        assert!(value.get("linkPath").is_some());
        assert!(value.get("linkType").is_some());
        assert!(value.get("createdAt").is_some());
    }
}
```

### 7.2 Backend error contract

Create one backend error type early so all modules return consistent failures. A minimal version can live in `src-tauri/src/models.rs` or a later `error.rs` if the implementation prefers.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    ConfigRead { path: PathBuf, message: String },
    ConfigWrite { path: PathBuf, message: String },
    InvalidMainSkillsDir { path: PathBuf, message: String },
    InvalidSkill { skill_dir_name: String, message: String },
    TargetNotFound { target_id: String },
    InvalidTargetDir { path: PathBuf, message: String },
    Conflict { path: PathBuf, message: String },
    MissingLink { path: PathBuf },
    MismatchedLink { path: PathBuf, expected: PathBuf, actual: Option<PathBuf> },
    UnsafeDeleteRefused { path: PathBuf, message: String },
    ConfirmationRequired { message: String },
    Io { path: Option<PathBuf>, message: String },
}

impl AppError {
    pub fn to_dto(&self) -> AppErrorDto {
        match self {
            AppError::Conflict { path, message } => AppErrorDto {
                code: "conflict".to_string(),
                message: format!("{}: {}", message, path.display()),
            },
            AppError::MissingLink { path } => AppErrorDto {
                code: "missingLink".to_string(),
                message: format!("安装记录存在，但链接已不存在：{}", path.display()),
            },
            AppError::MismatchedLink { path, expected, actual } => AppErrorDto {
                code: "mismatchedLink".to_string(),
                message: format!(
                    "链接目标异常：{}，期望指向 {}，实际指向 {:?}",
                    path.display(),
                    expected.display(),
                    actual
                ),
            },
            AppError::ConfirmationRequired { message } => AppErrorDto {
                code: "confirmationRequired".to_string(),
                message: message.clone(),
            },
            other => AppErrorDto {
                code: "error".to_string(),
                message: format!("{:?}", other),
            },
        }
    }
}
```

Tauri commands should return `Result<T, AppErrorDto>`, not raw `AppError` or panics.

### 7.3 Config store contract

Create `src-tauri/src/config_store.rs` with these public methods:

```rust
use crate::models::{AppConfig, AppError};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigStore {
    config_path: PathBuf,
}

impl ConfigStore {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub fn load(&self) -> Result<AppConfig, AppError> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }

        let raw = fs::read_to_string(&self.config_path).map_err(|err| AppError::ConfigRead {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        serde_json::from_str(&raw).map_err(|err| AppError::ConfigRead {
            path: self.config_path.clone(),
            message: err.to_string(),
        })
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), AppError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|err| AppError::ConfigWrite {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }

        let tmp_path = self.config_path.with_extension("json.tmp");
        let raw = serde_json::to_string_pretty(config).map_err(|err| AppError::ConfigWrite {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        fs::write(&tmp_path, raw).map_err(|err| AppError::ConfigWrite {
            path: tmp_path.clone(),
            message: err.to_string(),
        })?;

        fs::rename(&tmp_path, &self.config_path).map_err(|err| AppError::ConfigWrite {
            path: self.config_path.clone(),
            message: err.to_string(),
        })?;

        Ok(())
    }
}
```

Required tests:

```rust
#[test]
fn missing_config_returns_default() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ConfigStore::new(temp.path().join("config.json"));

    let config = store.load().expect("load default");

    assert_eq!(config.version, 1);
    assert!(config.settings.main_skills_dir.is_none());
    assert!(config.targets.is_empty());
    assert!(config.installations.is_empty());
}

#[test]
fn save_then_load_round_trips_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = ConfigStore::new(temp.path().join("config.json"));
    let mut config = AppConfig::default();
    config.settings.main_skills_dir = Some(temp.path().join("main-skills"));

    store.save(&config).expect("save config");
    let loaded = store.load().expect("load config");

    assert_eq!(loaded.settings.main_skills_dir, config.settings.main_skills_dir);
}

#[test]
fn malformed_config_returns_error_without_overwrite() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("config.json");
    std::fs::write(&path, "{not json").expect("write malformed json");
    let store = ConfigStore::new(path.clone());

    let error = store.load().expect_err("malformed config should fail");

    assert!(matches!(error, AppError::ConfigRead { .. }));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "{not json");
}
```

### 7.4 Skill library contract

Create `src-tauri/src/skill_library.rs` with a public function:

```rust
pub fn list_skills(main_dir: Option<&std::path::Path>) -> Result<Vec<SkillView>, AppError>
```

Rules:

- `None` returns `Ok(vec![])`.
- Missing configured directory returns `AppError::InvalidMainSkillsDir`.
- Regular files under the main directory are ignored.
- Direct child directories are converted into `SkillView`.
- Invalid directories are returned, not thrown away.

Frontmatter parsing requirements:

```rust
fn parse_skill_frontmatter(raw: &str) -> Result<SkillMetadata, Vec<String>>
```

Expected metadata struct:

```rust
#[derive(Debug, Deserialize)]
struct SkillMetadata {
    name: Option<String>,
    description: Option<String>,
}
```

Required validation error strings:

- `Missing SKILL.md`
- `Missing frontmatter`
- `Missing frontmatter.name`
- `Missing frontmatter.description`

Required tests:

```rust
#[test]
fn valid_skill_is_returned_as_installable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let skill_dir = temp.path().join("brainstorming");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: brainstorming\ndescription: Explore ideas.\n---\n\n# Skill\n",
    ).expect("write skill");

    let skills = list_skills(Some(temp.path())).expect("list skills");

    assert_eq!(skills.len(), 1);
    assert!(skills[0].valid);
    assert_eq!(skills[0].dir_name, "brainstorming");
    assert_eq!(skills[0].name.as_deref(), Some("brainstorming"));
}

#[test]
fn missing_skill_md_is_invalid_with_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("broken-skill")).expect("create skill dir");

    let skills = list_skills(Some(temp.path())).expect("list skills");

    assert_eq!(skills.len(), 1);
    assert!(!skills[0].valid);
    assert!(skills[0].validation_errors.contains(&"Missing SKILL.md".to_string()));
}
```

### 7.5 Filesystem adapter contract

Create `src-tauri/src/fs_adapter.rs` with this public API:

```rust
use crate::models::{AppError, LinkType};
use std::path::{Path, PathBuf};

pub fn default_link_type() -> LinkType;

pub fn path_exists(path: &Path) -> bool;

pub fn is_dir(path: &Path) -> bool;

pub fn create_dir_link(source: &Path, link_path: &Path, link_type: LinkType) -> Result<(), AppError>;

pub fn link_target(path: &Path) -> Result<Option<PathBuf>, AppError>;

pub fn remove_recorded_link(link_path: &Path, expected_target: &Path) -> Result<(), AppError>;

pub fn delete_real_dir(path: &Path) -> Result<(), AppError>;
```

Safety behavior for `remove_recorded_link`:

```rust
pub fn remove_recorded_link(link_path: &Path, expected_target: &Path) -> Result<(), AppError> {
    if !link_path.exists() {
        return Err(AppError::MissingLink { path: link_path.to_path_buf() });
    }

    let actual = link_target(link_path)?;
    if actual.as_deref() != Some(expected_target) {
        return Err(AppError::MismatchedLink {
            path: link_path.to_path_buf(),
            expected: expected_target.to_path_buf(),
            actual,
        });
    }

    std::fs::remove_dir(link_path).map_err(|err| AppError::Io {
        path: Some(link_path.to_path_buf()),
        message: err.to_string(),
    })
}
```

Required tests:

```rust
#[test]
fn refuses_to_remove_unknown_real_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let real_dir = temp.path().join("skill");
    std::fs::create_dir_all(&real_dir).expect("create real dir");

    let result = remove_recorded_link(&real_dir, temp.path().join("source").as_path());

    assert!(result.is_err());
    assert!(real_dir.exists());
}

#[test]
fn refuses_to_remove_unknown_regular_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_path = temp.path().join("skill");
    std::fs::write(&file_path, "not a link").expect("write file");

    let result = remove_recorded_link(&file_path, temp.path().join("source").as_path());

    assert!(result.is_err());
    assert!(file_path.exists());
}
```

On Windows, validate the junction implementation manually even if unit tests pass. Junction behavior can differ from symlink behavior, especially around target resolution.

### 7.6 Link installer contract

Create `src-tauri/src/link_installer.rs` with these public functions:

```rust
pub fn install_skill(
    config: &mut AppConfig,
    target_id: &str,
    skill_dir_name: &str,
    skills: &[SkillView],
) -> Result<(), AppError>;

pub fn uninstall_skill(
    config: &mut AppConfig,
    target_id: &str,
    skill_dir_name: &str,
) -> Result<(), AppError>;

pub fn compute_target_skill_states(
    config: &AppConfig,
    target_id: &str,
    skills: &[SkillView],
) -> Result<Vec<SkillWithTargetState>, AppError>;
```

Install pseudocode:

```rust
pub fn install_skill(
    config: &mut AppConfig,
    target_id: &str,
    skill_dir_name: &str,
    skills: &[SkillView],
) -> Result<(), AppError> {
    let target = config.targets.iter()
        .find(|target| target.id == target_id)
        .ok_or_else(|| AppError::TargetNotFound { target_id: target_id.to_string() })?;

    let skill = skills.iter()
        .find(|skill| skill.dir_name == skill_dir_name)
        .ok_or_else(|| AppError::InvalidSkill {
            skill_dir_name: skill_dir_name.to_string(),
            message: "Skill not found in main skills directory".to_string(),
        })?;

    if !skill.valid {
        return Err(AppError::InvalidSkill {
            skill_dir_name: skill_dir_name.to_string(),
            message: skill.validation_errors.join(", "),
        });
    }

    let link_path = target.skills_dir.join(&skill.dir_name);

    if link_path.exists() {
        let is_recorded = config.installations.iter().any(|installation| {
            installation.target_id == target_id
                && installation.skill_dir_name == skill_dir_name
                && installation.link_path == link_path
                && installation.source_path == skill.path
        });

        if is_recorded {
            return Ok(());
        }

        return Err(AppError::Conflict {
            path: link_path,
            message: "目标目录已存在同名内容，软件不会覆盖或接管".to_string(),
        });
    }

    let link_type = crate::fs_adapter::default_link_type();
    crate::fs_adapter::create_dir_link(&skill.path, &link_path, link_type.clone())?;

    config.installations.push(Installation {
        id: crate::models::new_id("inst"),
        skill_dir_name: skill.dir_name.clone(),
        skill_name: skill.name.clone().unwrap_or_else(|| skill.dir_name.clone()),
        source_path: skill.path.clone(),
        target_id: target_id.to_string(),
        link_path,
        link_type,
        created_at: crate::models::now_string(),
    });

    Ok(())
}
```

The helper names `new_id` and `now_string` may be implemented in `models.rs` or a small utility module. If a dependency is used instead, keep call sites equally explicit.

### 7.7 Skill remover contract

Create `src-tauri/src/skill_remover.rs` with this public function:

```rust
pub fn delete_main_skill(
    config: &mut AppConfig,
    skill_dir_name: &str,
    confirmed: bool,
) -> Result<DeleteMainSkillResult, AppError>
```

Result model:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMainSkillResult {
    pub deleted_skill_dir_name: String,
    pub removed_link_count: usize,
}
```

Required behavior pseudocode:

```rust
pub fn delete_main_skill(
    config: &mut AppConfig,
    skill_dir_name: &str,
    confirmed: bool,
) -> Result<DeleteMainSkillResult, AppError> {
    if !confirmed {
        return Err(AppError::ConfirmationRequired {
            message: "删除主目录 skill 不可恢复，需要确认".to_string(),
        });
    }

    let main_dir = config.settings.main_skills_dir.clone().ok_or_else(|| {
        AppError::InvalidMainSkillsDir {
            path: PathBuf::new(),
            message: "Main skills directory is not configured".to_string(),
        }
    })?;
    let source_path = main_dir.join(skill_dir_name);

    let related: Vec<Installation> = config.installations.iter()
        .filter(|installation| installation.skill_dir_name == skill_dir_name || installation.source_path == source_path)
        .cloned()
        .collect();

    for installation in &related {
        crate::fs_adapter::remove_recorded_link(&installation.link_path, &installation.source_path)?;
    }

    crate::fs_adapter::delete_real_dir(&source_path)?;

    config.installations.retain(|installation| {
        installation.skill_dir_name != skill_dir_name && installation.source_path != source_path
    });

    Ok(DeleteMainSkillResult {
        deleted_skill_dir_name: skill_dir_name.to_string(),
        removed_link_count: related.len(),
    })
}
```

Required abort test:

```rust
#[test]
fn delete_main_skill_aborts_when_recorded_link_cleanup_fails() {
    let temp = tempfile::tempdir().expect("tempdir");
    let main = temp.path().join("main");
    let source = main.join("skill-a");
    std::fs::create_dir_all(&source).expect("create source");
    std::fs::write(source.join("SKILL.md"), "---\nname: skill-a\ndescription: A.\n---\n").unwrap();

    let target = temp.path().join("target");
    std::fs::create_dir_all(&target).expect("create target");
    let fake_link_path = target.join("skill-a");
    std::fs::create_dir_all(&fake_link_path).expect("create real dir that is not a link");

    let mut config = AppConfig::default();
    config.settings.main_skills_dir = Some(main);
    config.installations.push(Installation {
        id: "inst_1".to_string(),
        skill_dir_name: "skill-a".to_string(),
        skill_name: "skill-a".to_string(),
        source_path: source.clone(),
        target_id: "target_1".to_string(),
        link_path: fake_link_path,
        link_type: LinkType::Symlink,
        created_at: "2026-06-23T00:00:00Z".to_string(),
    });

    let result = delete_main_skill(&mut config, "skill-a", true);

    assert!(result.is_err());
    assert!(source.exists(), "source skill must not be deleted when cleanup fails");
    assert_eq!(config.installations.len(), 1, "record must be preserved on failure");
}
```

---

## 8. Detailed frontend contracts

This section defines the frontend shape that should be implemented after backend commands exist. The frontend must remain a thin presentation and interaction layer. It should not decide whether a filesystem operation is safe; it should only display backend-computed state and call backend commands.

### 8.1 TypeScript model sketch

Create `src/model/types.ts` with these types. Keep string values aligned with Rust Serde enum output.

```ts
export type LinkStrategy = 'auto';
export type LinkType = 'junction' | 'symlink';

export type SkillInstallState =
  | 'notInstalled'
  | 'installed'
  | 'conflict'
  | 'missing'
  | 'mismatch'
  | 'sourceMissing'
  | 'invalidSkill';

export interface Settings {
  mainSkillsDir: string | null;
  linkStrategy: LinkStrategy;
}

export interface Target {
  id: string;
  name: string;
  skillsDir: string;
  createdAt: string;
  updatedAt: string;
}

export interface Installation {
  id: string;
  skillDirName: string;
  skillName: string;
  sourcePath: string;
  targetId: string;
  linkPath: string;
  linkType: LinkType;
  createdAt: string;
}

export interface AppConfig {
  version: number;
  settings: Settings;
  targets: Target[];
  installations: Installation[];
}

export interface SkillView {
  dirName: string;
  name: string | null;
  description: string | null;
  path: string;
  valid: boolean;
  validationErrors: string[];
}

export interface SkillWithTargetState {
  skill: SkillView;
  state: SkillInstallState;
  message: string | null;
}

export interface AppState {
  config: AppConfig;
  skills: SkillView[];
  selectedTargetId: string | null;
  selectedTargetSkills: SkillWithTargetState[];
}

export interface AppErrorDto {
  code: string;
  message: string;
}
```

### 8.2 Command wrapper contract

Create `src/api/commands.ts`. It should be the only frontend file importing Tauri `invoke`.

```ts
import { invoke } from '@tauri-apps/api/core';
import type { AppState } from '../model/types';

export async function getAppState(selectedTargetId?: string | null): Promise<AppState> {
  return invoke<AppState>('get_app_state', { selectedTargetId: selectedTargetId ?? null });
}

export async function setMainSkillsDir(path: string): Promise<AppState> {
  return invoke<AppState>('set_main_skills_dir', { path });
}

export async function addTarget(name: string, skillsDir: string): Promise<AppState> {
  return invoke<AppState>('add_target', { name, skillsDir });
}

export async function updateTarget(targetId: string, name: string, skillsDir: string): Promise<AppState> {
  return invoke<AppState>('update_target', { targetId, name, skillsDir });
}

export async function deleteTarget(targetId: string, cleanupRecordedLinks: boolean): Promise<AppState> {
  return invoke<AppState>('delete_target', { targetId, cleanupRecordedLinks });
}

export async function installSkill(targetId: string, skillDirName: string): Promise<AppState> {
  return invoke<AppState>('install_skill', { targetId, skillDirName });
}

export async function uninstallSkill(targetId: string, skillDirName: string): Promise<AppState> {
  return invoke<AppState>('uninstall_skill', { targetId, skillDirName });
}

export async function deleteMainSkill(skillDirName: string, confirmed: boolean): Promise<AppState> {
  return invoke<AppState>('delete_main_skill', { skillDirName, confirmed });
}
```

If `open_path` is included, add it as a convenience command only:

```ts
export async function openPath(path: string): Promise<void> {
  return invoke<void>('open_path', { path });
}
```

### 8.3 App state contract

`src/App.tsx` owns these state values:

```ts
const [appState, setAppState] = useState<AppState | null>(null);
const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
const [loading, setLoading] = useState(true);
const [error, setError] = useState<string | null>(null);
const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);
const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);
```

Required helper functions:

```ts
async function refresh(nextSelectedTargetId = selectedTargetId): Promise<void> {
  setLoading(true);
  try {
    const next = await getAppState(nextSelectedTargetId);
    setAppState(next);
    setSelectedTargetId(next.selectedTargetId);
    setError(null);
  } catch (err) {
    setError(errorMessage(err));
  } finally {
    setLoading(false);
  }
}

function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return '操作失败，请查看日志或重试。';
}
```

Install/uninstall toggle behavior:

```ts
async function handleToggleSkill(skillDirName: string, state: SkillInstallState): Promise<void> {
  if (!selectedTargetId) return;
  const key = `${selectedTargetId}:${skillDirName}`;
  setPendingSkillKey(key);

  try {
    if (state === 'notInstalled') {
      setAppState(await installSkill(selectedTargetId, skillDirName));
    } else if (state === 'installed') {
      setAppState(await uninstallSkill(selectedTargetId, skillDirName));
    }
    setError(null);
  } catch (err) {
    setError(errorMessage(err));
    await refresh(selectedTargetId);
  } finally {
    setPendingSkillKey(null);
  }
}
```

### 8.4 Component contracts

#### `Sidebar.tsx`

Props:

```ts
interface SidebarProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  targets: Target[];
  selectedTargetId: string | null;
  onSelectTarget: (targetId: string) => void;
  onAddTarget: () => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
  onSetMainSkillsDir: () => void;
}
```

Responsibilities:

- Render main directory path or `未设置主目录`.
- Render valid/invalid skill counts.
- Render target list.
- Delegate all actions to parent callbacks.

#### `TargetDetail.tsx`

Props:

```ts
interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillDirName: string, state: SkillInstallState) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
}
```

Responsibilities:

- If target is null, show empty state.
- Render target name and skills directory.
- Render `SkillRow` for each backend-provided skill state.
- Keep row ordering stable, preferably by skill directory name.

#### `SkillRow.tsx`

Props:

```ts
interface SkillRowProps {
  item: SkillWithTargetState;
  pending: boolean;
  onToggle: (skillDirName: string, state: SkillInstallState) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
}
```

Safety UI rules:

- Toggle enabled only for `notInstalled` and `installed`.
- Toggle checked only for `installed`.
- Toggle disabled for `conflict`, `missing`, `mismatch`, `sourceMissing`, and `invalidSkill`.
- Delete-main-skill button should be separate from install toggle and visually marked as dangerous.
- Show `item.message` when present.

State label helper:

```ts
export function stateLabel(state: SkillInstallState): string {
  switch (state) {
    case 'notInstalled': return '未安装';
    case 'installed': return '已安装';
    case 'conflict': return '冲突';
    case 'missing': return '缺失';
    case 'mismatch': return '异常';
    case 'sourceMissing': return '源缺失';
    case 'invalidSkill': return '无效 skill';
  }
}

export function canToggle(state: SkillInstallState): boolean {
  return state === 'notInstalled' || state === 'installed';
}
```

#### `ConfirmDialog.tsx`

Props:

```ts
interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}
```

Responsibilities:

- Render nothing when `open` is false.
- Escape closes the dialog by calling `onCancel`.
- Confirm button calls `onConfirm`.
- Danger style is used for irreversible main skill deletion.

### 8.5 Minimum CSS contract

`src/styles.css` should include these layout classes or equivalents:

```css
.app-shell {
  display: grid;
  grid-template-columns: 320px 1fr;
  min-height: 100vh;
}

.sidebar {
  border-right: 1px solid #e5e7eb;
  padding: 16px;
  background: #f9fafb;
}

.main-panel {
  padding: 24px;
}

.skill-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 12px;
  align-items: center;
  padding: 12px;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  margin-bottom: 8px;
}

.status-badge {
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 12px;
}

.status-conflict,
.status-mismatch,
.status-missing,
.status-sourceMissing,
.status-invalidSkill {
  background: #fee2e2;
  color: #991b1b;
}

.status-installed {
  background: #dcfce7;
  color: #166534;
}

.status-notInstalled {
  background: #e5e7eb;
  color: #374151;
}

.error-banner {
  background: #fef2f2;
  color: #991b1b;
  border: 1px solid #fecaca;
  padding: 12px;
  border-radius: 8px;
  margin-bottom: 16px;
}

.danger-button {
  color: #991b1b;
  border-color: #fecaca;
}
```

### 8.6 Frontend tests detail

Use mocked command wrappers. Example fixture:

```ts
export const appStateFixture: AppState = {
  config: {
    version: 1,
    settings: { mainSkillsDir: '/tmp/main-skills', linkStrategy: 'auto' },
    targets: [
      {
        id: 'target_1',
        name: 'Claude Global',
        skillsDir: '/tmp/target',
        createdAt: '2026-06-23T00:00:00Z',
        updatedAt: '2026-06-23T00:00:00Z',
      },
    ],
    installations: [],
  },
  skills: [],
  selectedTargetId: 'target_1',
  selectedTargetSkills: [
    {
      skill: {
        dirName: 'brainstorming',
        name: 'brainstorming',
        description: 'Explore ideas.',
        path: '/tmp/main-skills/brainstorming',
        valid: true,
        validationErrors: [],
      },
      state: 'notInstalled',
      message: null,
    },
  ],
};
```

Required test examples:

```ts
it('renders target skills for the selected target', async () => {
  vi.mocked(getAppState).mockResolvedValue(appStateFixture);

  render(<App />);

  expect(await screen.findByText('brainstorming')).toBeInTheDocument();
  expect(screen.getByText('Explore ideas.')).toBeInTheDocument();
  expect(screen.getByText('未安装')).toBeInTheDocument();
});

it('calls install command when toggling a not installed skill', async () => {
  vi.mocked(getAppState).mockResolvedValue(appStateFixture);
  vi.mocked(installSkill).mockResolvedValue({
    ...appStateFixture,
    selectedTargetSkills: [
      { ...appStateFixture.selectedTargetSkills[0], state: 'installed' },
    ],
  });

  render(<App />);
  await screen.findByText('brainstorming');
  await userEvent.click(screen.getByRole('checkbox', { name: /brainstorming/i }));

  expect(installSkill).toHaveBeenCalledWith('target_1', 'brainstorming');
});

it('does not allow toggling a conflict state', () => {
  const conflictState = {
    ...appStateFixture,
    selectedTargetSkills: [
      {
        ...appStateFixture.selectedTargetSkills[0],
        state: 'conflict' as const,
        message: '目标目录已存在同名内容',
      },
    ],
  };

  render(<TargetDetail
    target={conflictState.config.targets[0]}
    skills={conflictState.selectedTargetSkills}
    pendingSkillKey={null}
    onToggleSkill={vi.fn()}
    onDeleteMainSkill={vi.fn()}
  />);

  expect(screen.getByRole('checkbox', { name: /brainstorming/i })).toBeDisabled();
  expect(screen.getByText('冲突')).toBeInTheDocument();
});
```

---

## 9. Tauri command implementation detail

### 9.1 Command state loading pattern

Each mutating command should follow this pattern:

1. Resolve config path.
2. Load config.
3. Load current skills if needed.
4. Mutate config through domain function.
5. Save config.
6. Return fresh `AppState`.

Sketch:

```rust
#[tauri::command]
pub fn install_skill(
    app: tauri::AppHandle,
    target_id: String,
    skill_dir_name: String,
) -> Result<AppState, AppErrorDto> {
    run_with_config(app, |config| {
        let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())?;
        crate::link_installer::install_skill(config, &target_id, &skill_dir_name, &skills)?;
        Ok(())
    }, Some(target_id)).map_err(|err| err.to_dto())
}
```

Shared helper sketch:

```rust
fn run_with_config<F>(
    app: tauri::AppHandle,
    mutate: F,
    selected_target_id: Option<String>,
) -> Result<AppState, AppError>
where
    F: FnOnce(&mut AppConfig) -> Result<(), AppError>,
{
    let store = store_from_app(&app)?;
    let mut config = store.load()?;
    mutate(&mut config)?;
    store.save(&config)?;
    build_app_state(config, selected_target_id)
}
```

For read-only commands, do not save config.

### 9.2 `build_app_state` contract

```rust
pub fn build_app_state(
    config: AppConfig,
    selected_target_id: Option<String>,
) -> Result<AppState, AppError> {
    let skills = crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())?;
    let selected = selected_target_id
        .or_else(|| config.targets.first().map(|target| target.id.clone()));

    let selected_target_skills = match selected.as_deref() {
        Some(target_id) => crate::link_installer::compute_target_skill_states(&config, target_id, &skills)?,
        None => Vec::new(),
    };

    Ok(AppState {
        config,
        skills,
        selected_target_id: selected,
        selected_target_skills,
    })
}
```

### 9.3 Command list details

- `get_app_state(selectedTargetId)`
  - Loads config.
  - Builds state with selected target ID or first target.
  - Does not mutate config.

- `set_main_skills_dir(path)`
  - Validates path exists and is a directory.
  - Updates `settings.mainSkillsDir`.
  - Does not scan target directories.
  - Does not remove existing installation records automatically. Existing records may become source-missing/mismatch and should be shown as abnormal.

- `add_target(name, skillsDir)`
  - Validates name is non-empty.
  - Validates path exists and is directory.
  - Adds target config.
  - Returns app state with new target selected.

- `update_target(targetId, name, skillsDir)`
  - Validates target exists.
  - Validates name/path.
  - Updates target config.
  - Does not move existing links automatically.
  - Existing installation records may become missing/mismatch if target path changes. This is acceptable in v1 and should be surfaced.

- `delete_target(targetId, cleanupRecordedLinks)`
  - If target has installation records and cleanup flag is false, return a user-safe error.
  - If cleanup flag is true, uninstall each recorded link for that target using the same safe link deletion rules.
  - Delete target config only after cleanup succeeds.
  - Never delete the target directory itself.

- `install_skill(targetId, skillDirName)`
  - Delegates to `link_installer::install_skill`.

- `uninstall_skill(targetId, skillDirName)`
  - Delegates to `link_installer::uninstall_skill`.

- `delete_main_skill(skillDirName, confirmed)`
  - Delegates to `skill_remover::delete_main_skill`.
  - Confirmation must be true.

- `open_path(path)`
  - Optional convenience command.
  - Must not be required for MVP correctness.

### 9.4 Command registration

`src-tauri/src/main.rs` should register commands explicitly:

```rust
mod commands;
mod config_store;
mod fs_adapter;
mod link_installer;
mod models;
mod skill_library;
mod skill_remover;
mod target_registry;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_main_skills_dir,
            commands::add_target,
            commands::update_target,
            commands::delete_target,
            commands::install_skill,
            commands::uninstall_skill,
            commands::delete_main_skill,
            commands::open_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

If `open_path` is deferred, remove it from both the command list and frontend wrapper in the same commit.

---

## 10. Documentation deliverables

### 10.1 README minimum content

Create `README.md` with:

```markdown
# Skills Sync Manager

A local desktop app for managing one main skills directory and installing selected skills into multiple local target directories through directory links.

## Safety model

- The app does not scan the machine for agent directories.
- The app does not overwrite existing target content.
- The app only uninstalls links that it created and recorded.
- Main skill deletion is irreversible in v1 and requires confirmation.

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

## Link behavior

- Windows: junction by default.
- macOS/Linux: directory symlink by default.
```

### 10.2 Test document

Create `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md` with sections:

- Unit tests
- Frontend tests
- Backend filesystem tests
- Manual Windows verification
- Manual macOS verification
- Manual Linux verification
- Final smoke test

Minimum manual smoke test:

```markdown
## Final smoke test

- [ ] Create temporary main skills directory.
- [ ] Create valid skill with `SKILL.md` frontmatter `name` and `description`.
- [ ] Create invalid skill missing `description`.
- [ ] Create two temporary target directories.
- [ ] Set main directory in app.
- [ ] Add both target directories.
- [ ] Install valid skill into target A.
- [ ] Install valid skill into target B.
- [ ] Edit source `SKILL.md`; confirm target A and B reflect the change.
- [ ] Uninstall from target A; confirm source and target B remain.
- [ ] Create same-name real directory in target A; confirm install is blocked.
- [ ] Delete main skill after confirmation; confirm recorded links are cleaned.
```

### 10.3 Release checklist

Create `docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-release-checklist.md` only when packaging/distribution work starts. For coding-only MVP, it can remain absent.

If created, include:

- Windows package built
- macOS package built
- Linux package built
- Manual verification completed on each platform
- Destructive actions tested only on temporary directories
- README updated
- Known limitations listed

---

## 11. Completion checklist

Before marking this plan implemented, verify every item below:

- [ ] The app launches as a Tauri desktop app.
- [ ] The app has no deployed service dependency.
- [ ] A single main skills directory can be configured.
- [ ] Valid skills require `SKILL.md` with `name` and `description`.
- [ ] Invalid skills are visible with reasons and cannot be installed.
- [ ] Targets can be added, edited, and deleted without deleting target directories.
- [ ] Installing creates a recorded directory link.
- [ ] Windows uses junction by default.
- [ ] macOS/Linux use directory symlink by default.
- [ ] Unknown same-name content blocks installation.
- [ ] Uninstall removes only recorded links.
- [ ] Missing and mismatched recorded links are surfaced as abnormal states.
- [ ] Main skill deletion requires confirmation.
- [ ] Main skill deletion cleans recorded links before deleting source.
- [ ] Main skill deletion aborts if recorded-link cleanup fails.
- [ ] Config persists across app restarts.
- [ ] Frontend tests pass.
- [ ] Rust tests pass.
- [ ] Production frontend build passes.
- [ ] README documents safety behavior and dev commands.
- [ ] Manual smoke test was run with temporary directories.

---

## 12. Suggested commit sequence

Use this sequence unless implementation work reveals a better split:

1. `chore: scaffold tauri react app`
2. `feat: define shared domain models`
3. `feat: add json config store`
4. `feat: validate skill library`
5. `feat: manage target registry`
6. `feat: add filesystem adapter`
7. `feat: install skills with recorded links`
8. `feat: uninstall recorded skill links safely`
9. `feat: delete main skills after link cleanup`
10. `feat: expose tauri commands`
11. `feat: build target-centered ui layout`
12. `feat: wire immediate skill toggles`
13. `feat: confirm main skill deletion`
14. `test: cover frontend interactions`
15. `test: cover backend safety flows`
16. `docs: add cross-platform verification checklist`
17. `feat: polish safety state messaging`
18. `docs: finalize skills manager mvp docs`

Every commit should include:

```text
Co-Authored-By: Claude <noreply@anthropic.com>
```
