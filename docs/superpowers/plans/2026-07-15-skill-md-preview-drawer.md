# Skill Markdown Preview Drawer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open a compact right-hand drawer showing a rendered `SKILL.md` when the user clicks a skill name on TargetDetail or Skill Hub (installed + discover).

**Architecture:** Add Rust `read_skill_markdown` that resolves installed paths, Git repo-cache/single-file fetch, or Hub skill archive extraction; return stripped body + header meta. Frontend hosts one shared `SkillPreviewDrawer` at App level (single business-drawer rule with source manage), renders with `react-markdown` + `remark-gfm`, and wires name clicks on `SkillRow` / `SkillCard`.

**Tech Stack:** Rust/Tauri, React 18, TypeScript, Vitest, `react-markdown`, `remark-gfm`, existing drawer CSS/`useModalFocus` patterns.

## Global Constraints

- Drawer UI: compact ~420px, Option A from spec; Chinese copy「Skill 预览」「关闭」「重试」.
- Open affordance: skill **name** only; checkbox / multi-select / action buttons unchanged.
- Content: strip YAML frontmatter; render body only; header shows name + description.
- Git preview: never re-download whole repo; prefer `repo_cache`, else single-file fetch.
- Hub discover preview: use existing per-skill `download_archive`, extract `SKILL.md`.
- Only one business drawer at a time (preview ↔ source manage).
- Spec: `docs/superpowers/specs/2026-07-15-skill-md-preview-drawer-design.md`.

---

## File Map

| Path | Responsibility |
|------|----------------|
| Create `src-tauri/src/skill_markdown.rs` | Resolve + read + frontmatter split for preview |
| Modify `src-tauri/src/skill_library.rs` | Export/reuse `split_frontmatter` / preview parse helpers |
| Modify `src-tauri/src/models.rs` | Request/response DTOs + error mapping if needed |
| Modify `src-tauri/src/commands/skill_hub.rs` or `commands/mod.rs` | Tauri command `read_skill_markdown` |
| Modify `src-tauri/src/lib.rs` | `mod skill_markdown` + register command |
| Modify `src/model/types.ts` | TS DTOs |
| Modify `src/api/skillHub.ts` or `commands.ts` | `readSkillMarkdown` invoke |
| Create `src/components/SkillMarkdownView.tsx` | react-markdown wrapper |
| Create `src/components/SkillPreviewDrawer.tsx` | Drawer chrome + load/error |
| Modify `src/components/SkillRow.tsx` | Clickable name |
| Modify `src/components/skill-hub/SkillCard.tsx` | Clickable name + stopPropagation |
| Modify `src/components/TargetDetail.tsx` | Pass `onPreviewSkill` |
| Modify `src/components/skill-hub/SkillHubPage.tsx` | Preview callback; close source drawer when opening preview |
| Modify `src/App.tsx` | Own preview request state + mount drawer |
| Modify `src/styles/hub.css` / `target.css` | Drawer + name link styles |
| Tests per task | Rust unit + Vitest |

---

### Task 1: Backend — parse preview + read installed skills

**Files:**
- Create: `src-tauri/src/skill_markdown.rs`
- Modify: `src-tauri/src/skill_library.rs` (make body-aware parse reusable)
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs` (`mod skill_markdown`)
- Test: `skill_markdown.rs` `mod tests`

**Interfaces:**
- Produces:
  - `pub fn parse_skill_markdown_preview(raw: &str) -> SkillMarkdownParts` where `SkillMarkdownParts { title: Option<String>, description: Option<String>, markdown_body: String }`
  - `pub fn read_installed_skill_markdown(main_skills_dir: Option<&str>, storage_key: &str) -> Result<SkillMarkdownPreviewDto, AppError>`
  - DTO fields (camelCase serde): `title`, `description`, `markdownBody`, `origin` (`"mainLibrary"` | …)

- [ ] **Step 1: Write failing tests for frontmatter strip + installed read**

```rust
#[test]
fn parse_strips_frontmatter_and_keeps_body() {
    let raw = "---\nname: Demo\ndescription: Hello\n---\n\n# Title\n\nBody\n";
    let parts = parse_skill_markdown_preview(raw);
    assert_eq!(parts.title.as_deref(), Some("Demo"));
    assert_eq!(parts.description.as_deref(), Some("Hello"));
    assert!(parts.markdown_body.contains("# Title"));
    assert!(!parts.markdown_body.contains("description:"));
}

#[test]
fn read_installed_returns_main_library_origin() {
    // tempfile main dir with storage_key path + SKILL.md
    // assert origin == "mainLibrary" and body stripped
}
```

Reuse `split_frontmatter` logic: prefer exporting a new public helper from `skill_library` or duplicating minimally inside `skill_markdown` that calls into made-`pub(crate)` functions. Prefer `pub(crate) fn split_skill_md(raw: &str) -> (Option<SkillMetadata>, String)` in `skill_library.rs`.

- [ ] **Step 2: Run RED**

```bash
cargo test --manifest-path src-tauri/Cargo.toml parse_strips_frontmatter -- --nocapture
```

Expected: compile fail / test fail.

- [ ] **Step 3: Implement parse + installed reader**

```rust
// skill_markdown.rs
pub fn parse_skill_markdown_preview(raw: &str) -> SkillMarkdownParts { /* ... */ }

pub fn read_installed_skill_markdown(
    main_skills_dir: Option<&str>,
    storage_key: &str,
) -> Result<SkillMarkdownPreviewDto, AppError> {
    let skills = crate::skill_library::list_skills(main_skills_dir)?;
    let skill = skills
        .iter()
        .find(|s| s.storage_key == storage_key)
        .ok_or_else(|| AppError::Io {
            path: None,
            message: format!("未找到 skill：{storage_key}"),
        })?;
    let path = skill.path.join("SKILL.md");
    let raw = fs_adapter::read_to_string(&path).map_err(...)?;
    let parts = parse_skill_markdown_preview(&raw);
    Ok(SkillMarkdownPreviewDto {
        title: parts.title.unwrap_or_else(|| skill.dir_name.clone()),
        description: parts
            .description
            .or_else(|| skill.description.clone())
            .unwrap_or_default(),
        markdown_body: parts.markdown_body,
        origin: "mainLibrary".into(),
    })
}
```

Adapt to actual `fs_adapter` / `std::fs` patterns used in the crate. If skill has no valid path, still try `skill_storage::main_library_path(main_dir, storage_key).join("SKILL.md")`.

- [ ] **Step 4: GREEN**

```bash
cargo test --manifest-path src-tauri/Cargo.toml skill_markdown -- --nocapture
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/skill_markdown.rs src-tauri/src/skill_library.rs src-tauri/src/models.rs src-tauri/src/lib.rs
git commit -m "feat: parse and read installed skill markdown for preview"
```

---

### Task 2: Backend — discover Git + Hub resolution + Tauri command

**Files:**
- Modify: `src-tauri/src/skill_markdown.rs`
- Modify: `src-tauri/src/commands/skill_hub.rs` (or `commands/mod.rs`)
- Modify: `src-tauri/src/lib.rs` (handler registration)
- Optionally modify: `src-tauri/src/repo_cache.rs` / git client for single-file fetch helper
- Test: extend `skill_markdown` tests with mocks/temp cache + zip

**Interfaces:**
- Consumes: Task 1 parse/read helpers; `repo_cache::cache_dir` / tree paths; `skill_hub_client::download_archive`; `DiscoverableSkill` from `config.skill_discover_cache`
- Produces:
  - `pub fn read_skill_markdown(config: &AppConfig, app_data_dir: &Path, request: SkillMarkdownRequestDto) -> Result<SkillMarkdownPreviewDto, AppError>`
  - Tauri: `read_skill_markdown(app, request) -> Result<SkillMarkdownPreviewDto, AppErrorDto>`
  - Request DTO:
    ```rust
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum SkillMarkdownRequestDto {
        #[serde(rename = "installed")]
        Installed { storage_key: String },
        #[serde(rename = "discover")]
        Discover { discover_key: String },
    }
    ```
    (If tagged enums fight Tauri/serde, use `{ kind: string, storageKey?: string, discoverKey?: string }` flat struct instead — pick one and keep TS identical.)

- [ ] **Step 1: Failing tests**

1. Git discover with file present under fake `repo-cache/.../tree/.../SKILL.md` → `origin == "repoCache"`, no download hook called.
2. Git discover missing cache → single-file fetch hook called once; `origin == "remoteFile"`.
3. Hub discover → mocked archive zip containing `SKILL.md` → `origin == "hubArchive"`.
4. Unknown discover key → error.

- [ ] **Step 2: RED**

```bash
cargo test --manifest-path src-tauri/Cargo.toml skill_markdown -- --nocapture
```

- [ ] **Step 3: Implement discover resolution**

```rust
// Pseudocode
Discover { discover_key } => {
  let skill = config.skill_discover_cache.skills.iter()
    .find(|s| s.key == discover_key)
    .ok_or(...)?;
  if skill.source == "skillhub" {
    let endpoint = find_endpoint(config, &skill.hub_endpoint_id)?;
    let zip = skill_hub_client::download_archive(&endpoint.base_url, &skill.hub_skill_group, &skill.hub_skill_id)?;
    let raw = extract_skill_md_from_zip(&zip)?;
    // parse → origin hubArchive
  } else {
    // build RepoRef from skill fields; locate tree via repo_cache
    let candidate = repo_tree.join(&skill.directory).join("SKILL.md");
    // also try install_dir_name layouts used by discover scanner
    if candidate.exists() { read local; origin repoCache }
    else {
      let raw = fetch_remote_skill_md(&repo_ref, &relative_path)?; // single file
      origin remoteFile
    }
  }
}
```

Implement `fetch_remote_skill_md` using the smallest existing HTTP helper (GitHub raw URL / GitLab raw API). If a clean single-file helper does not exist, add a focused function in `skill_markdown.rs` or git client module — **do not** call `ensure_repo_tree` for the miss path.

For Hub zip extraction, follow patterns in `skill_install` / `skill_downloader::extract_zip_file` but read file bytes without installing into main library (temp dir OK).

- [ ] **Step 4: Wire command**

```rust
#[tauri::command]
pub fn read_skill_markdown(
    app: tauri::AppHandle,
    request: SkillMarkdownRequestDto,
) -> Result<SkillMarkdownPreviewDto, AppErrorDto> {
    let store = store_from_app(&app).map_err(|e| e.to_dto())?;
    let config = store.load().map_err(|e| e.to_dto())?;
    let app_data = app_data_dir_from_app(&app)?; // match existing helper name
    crate::skill_markdown::read_skill_markdown(&config, &app_data, request).map_err(|e| e.to_dto())
}
```

Register in `generate_handler![]`.

- [ ] **Step 5: GREEN + commit**

```bash
cargo test --manifest-path src-tauri/Cargo.toml skill_markdown read_skill -- --nocapture
git add src-tauri/src/skill_markdown.rs src-tauri/src/commands src-tauri/src/lib.rs src-tauri/src/models.rs
git commit -m "feat: resolve discover skill markdown for preview command"
```

---

### Task 3: Frontend types, API, markdown dependencies

**Files:**
- Modify: `package.json` / `package-lock.json` (via npm install)
- Modify: `src/model/types.ts`
- Modify: `src/api/skillHub.ts` (preferred if hub-related) **or** `src/api/commands.ts`
- Create: `src/test/readSkillMarkdownApi.test.ts` only if valuable; otherwise skip and cover via drawer tests

**Interfaces:**
- Produces:
```ts
export type SkillMarkdownRequest =
  | { kind: 'installed'; storageKey: string }
  | { kind: 'discover'; discoverKey: string };

export interface SkillMarkdownPreview {
  title: string;
  description: string;
  markdownBody: string;
  origin: 'mainLibrary' | 'repoCache' | 'remoteFile' | 'hubArchive';
}

export async function readSkillMarkdown(
  request: SkillMarkdownRequest,
): Promise<SkillMarkdownPreview>
```

- [ ] **Step 1: Install deps**

```bash
npm install react-markdown remark-gfm
```

- [ ] **Step 2: Add types + invoke wrapper** matching Rust serde shape exactly.

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json src/model/types.ts src/api/skillHub.ts
git commit -m "feat: add readSkillMarkdown API and markdown dependencies"
```

---

### Task 4: `SkillMarkdownView` + `SkillPreviewDrawer`

**Files:**
- Create: `src/components/SkillMarkdownView.tsx`
- Create: `src/components/SkillPreviewDrawer.tsx`
- Create: `src/test/SkillPreviewDrawer.test.tsx`
- Modify: `src/styles/hub.css` (drawer class reuse / `skill-preview-drawer`)

**Interfaces:**
- Consumes: `readSkillMarkdown`
- Produces:
```ts
export interface SkillPreviewDrawerProps {
  open: boolean;
  request: SkillMarkdownRequest | null;
  onClose: () => void;
}
```

- [ ] **Step 1: Failing drawer tests** (mock `readSkillMarkdown`)

- Opens with title「Skill 预览」and loads preview for request
- Shows skeleton/pending UI while promise pending
- Renders markdown heading text from body
- Error shows「重试」; retry calls API again
- Escape / overlay / 关闭 call `onClose`
- Changing `request` while open reloads (generation guard ignores stale)

Pattern: `src/test/SourceManageDrawer.test.tsx`.

- [ ] **Step 2: RED**

```bash
npm test -- --run src/test/SkillPreviewDrawer.test.tsx
```

- [ ] **Step 3: Implement components**

`SkillMarkdownView`:

```tsx
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

export function SkillMarkdownView({ markdown }: { markdown: string }) {
  return (
    <div className="skill-markdown-view">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
    </div>
  );
}
```

`SkillPreviewDrawer`: mirror `SourceManageDrawer` shell (`overlay drawer-overlay`, width ~420px, `useModalFocus` with Escape → `onClose` when idle). On `open && request`, fetch; show header title/description from response; body markdown or skeleton/error.

- [ ] **Step 4: GREEN + commit**

```bash
npm test -- --run src/test/SkillPreviewDrawer.test.tsx
git add src/components/SkillMarkdownView.tsx src/components/SkillPreviewDrawer.tsx src/test/SkillPreviewDrawer.test.tsx src/styles/hub.css
git commit -m "feat: add SkillPreviewDrawer with markdown rendering"
```

---

### Task 5: Wire TargetDetail + SkillRow + App host

**Files:**
- Modify: `src/components/SkillRow.tsx`
- Modify: `src/components/TargetDetail.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/target.css` (`.skill-name.skill-name-button` link style)
- Modify: `src/test/TargetDetail.test.tsx` / `SkillRow` tests if present
- Modify: `src/test/app.test.tsx` as needed

**Interfaces:**
- `SkillRowProps.onPreview?: (storageKey: string) => void`
- Name becomes `<button type="button" className="skill-name skill-name-link">` calling `onPreview?.(skillKey)`
- `TargetDetailProps.onPreviewSkill?: (storageKey: string) => void`
- App state:
```ts
const [skillPreview, setSkillPreview] = useState<SkillMarkdownRequest | null>(null);
// open when non-null
```

- [ ] **Step 1: Failing tests** — name click calls preview; checkbox click does not.

- [ ] **Step 2: Implement wiring** — mount `<SkillPreviewDrawer open={!!skillPreview} request={skillPreview} onClose={() => setSkillPreview(null)} />` near other global dialogs in `App.tsx`. Pass preview handler into `TargetDetail`.

- [ ] **Step 3: GREEN + commit**

```bash
npm test -- --run src/test/TargetDetail.test.tsx src/test/app.test.tsx
git commit -m "feat: open skill preview drawer from target skill names"
```

---

### Task 6: Wire Skill Hub cards + single-drawer rule

**Files:**
- Modify: `src/components/skill-hub/SkillCard.tsx`
- Modify: `src/components/skill-hub/SkillHubPage.tsx`
- Modify: `src/App.tsx` (pass `onPreviewSkill` / close source drawer callback)
- Modify: `src/test/SkillHubPage.test.tsx`

**Interfaces:**
- `SkillCard` adds `onPreview?: () => void`
- Title `<h3>` becomes button/link that `stopPropagation` + `onPreview?.()`
- Discover card body click still toggles selection; name does not toggle
- Installed mode: name previews; actions unchanged
- When opening preview from Hub: `setSourceDrawerOpen(false)` (and App opens preview)
- When opening source manage: App clears `skillPreview` (pass `onOpenSourceManage` wrapper)

Preferred coordination:
- App owns `skillPreview`
- SkillHubPage receives `onPreviewSkill(request)` and `previewOpen` or simply callbacks:
  - `onPreviewSkill` → App sets preview **and** Hub closes source drawer internally before calling up
  - `onOpenSources` already local — also call optional `onCloseSkillPreview()` from App

- [ ] **Step 1: Failing tests**

- Discover: click title opens preview callback with `{ kind:'discover', discoverKey }`
- Discover: click card body still selects
- Opening preview closes source drawer if open

- [ ] **Step 2: Implement**

```ts
// installed
onPreviewSkill?.({ kind: 'installed', storageKey: skill.storageKey })
// discover
onPreviewSkill?.({ kind: 'discover', discoverKey: skill.key })
```

- [ ] **Step 3: GREEN + commit**

```bash
npm test -- --run src/test/SkillHubPage.test.tsx src/test/SkillPreviewDrawer.test.tsx
git commit -m "feat: open skill preview from Skill Hub cards"
```

---

### Task 7: Full verification

- [ ] **Step 1:** `npm test`
- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] **Step 3:** `npm run build`
- [ ] **Step 4:** Manual smoke
  1. Target page: click name → drawer with rendered MD; checkbox still toggles install
  2. Hub installed: same
  3. Hub discover Git (cached): opens without obvious long wait
  4. Hub discover Hub skill: loading then content or clear error
  5. Open source manage, then preview a skill → source drawer closes
  6. Escape closes preview

- [ ] **Step 5:** Commit any fixes only if needed

---

## Self-Review

| Spec item | Task |
|-----------|------|
| Compact 420px drawer UI | Task 4 |
| Name click Target + Hub | Tasks 5–6 |
| `read_skill_markdown` installed/git/hub | Tasks 1–2 |
| react-markdown + remark-gfm | Tasks 3–4 |
| Strip frontmatter; header meta | Task 1 + 4 |
| No whole-repo redownload | Task 2 |
| Single business drawer | Task 6 |
| Loading/error/retry | Task 4 |
| Tests + verification | Tasks 1–7 |

No TBD placeholders. DTO field names (`storageKey`, `discoverKey`, `markdownBody`, `origin`) must stay consistent across Rust serde and TS.
