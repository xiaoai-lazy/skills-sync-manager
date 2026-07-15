# Skill Markdown Preview Drawer Design

## Goal

Let users open a skill’s `SKILL.md` in a right-hand drawer by clicking the skill name on both the target detail page and Skill Hub (installed and discoverable), with rendered Markdown preview and source-aware content loading that prefers local files and avoids re-downloading whole Git repositories.

## Scope

In scope:

1. Shared `SkillPreviewDrawer` (~420px, aligned with source-manage drawer).
2. Clickable skill **name** as the open affordance on TargetDetail rows and Skill Hub cards.
3. Backend `read_skill_markdown` that resolves content for installed and discoverable skills.
4. Read-only rendering via `react-markdown` + `remark-gfm`.
5. Strip YAML frontmatter from the body; show name/description in the drawer header.

Out of scope:

- Editing `SKILL.md`
- Source/raw toggle
- In-drawer heading outline / dual-pane TOC
- Replacing discover cache or changing install pipelines
- Preview from places other than TargetDetail and Skill Hub skill lists

## Background

Today skill lists show name and description only. `skill_library` parses frontmatter and discards the body. Discover stores metadata in `skill_discover_cache`; Git repos may already exist under `repo_cache`, while Hub discover only lists DTOs until install (install uses per-skill archive download). The only existing drawer pattern is `SourceManageDrawer` (overlay + ~420px right panel, z-index 100).

## Architecture

```
Click skill name (TargetDetail / SkillHub)
        │
        ▼
  SkillPreviewDrawer opens (skeleton while loading)
        │
        ▼
  read_skill_markdown(ref)
        │
        ├─ installed → main library path / SKILL.md
        ├─ discover/git → repo_cache hit, else single-file fetch
        └─ discover/hub → download skill archive, extract SKILL.md
        │
        ▼
  Parse frontmatter → header meta + markdownBody
  Render body with react-markdown + remark-gfm
```

| Unit | Responsibility |
|------|----------------|
| `read_skill_markdown` (Rust command) | Resolve path or fetch; return preview DTO |
| Frontmatter strip helper | Split YAML header from body; extract name/description |
| `SkillPreviewDrawer` | Drawer chrome, load/error/retry, focus/Escape/overlay |
| `SkillMarkdownView` | Read-only Markdown render |
| `SkillRow` / `SkillCard` | Name click opens preview; other controls stop propagation |

## UI (Option A — Compact Drawer)

- Width ~420px; right slide-over; dimmed overlay; match source-manage visual language.
- Header: eyebrow「Skill 预览」, title (name), description, close control.
- Body: scrollable rendered Markdown.
- Footer:「关闭」.
- Name affordance: link-like weight/underline + `cursor: pointer` so it is distinct from non-clickable description.
- Loading (>300ms): skeleton in the body region.
- Error: message +「重试」; close always available.
- Motion: 150–300ms slide/fade; respect `prefers-reduced-motion`.

### Open / close / stacking

- Open: click skill **name** only.
  - TargetDetail: checkbox and action buttons do not open preview.
  - Hub discover: name opens preview (`stopPropagation`); rest of card keeps multi-select.
  - Hub installed: name opens preview; update/delete unchanged.
- Close: header ✕, footer close, overlay click, Escape.
- Focus: `role="dialog"`, focus trap, restore focus to the name that opened it.
- Stacking: same overlay tier as source manage (`z-index: 100`). **Only one business drawer at a time** — opening the preview drawer replaces/closes an open source-manage drawer and vice versa.
- Switching skills while open: keep drawer open; reload content for the new ref (skeleton → content).

## Content Loading

### Request / response

```ts
type SkillMarkdownRequest =
  | { kind: 'installed'; storageKey: string }
  | { kind: 'discover'; discoverKey: string };

interface SkillMarkdownPreview {
  title: string;
  description: string;
  markdownBody: string;
  origin: 'mainLibrary' | 'repoCache' | 'remoteFile' | 'hubArchive';
}
```

### Resolution rules

1. **Installed** (`kind: 'installed'`): locate skill via main library / `storageKey`, read `{path}/SKILL.md` from disk.
2. **Discover · GitHub/GitLab**: resolve directory under existing `repo_cache` tree and read `SKILL.md` when present (`origin: repoCache`). If missing, fetch **only that file** via provider raw/contents API (`origin: remoteFile`) — do **not** re-download the whole repository for preview.
3. **Discover · Skill Hub**: Hub list API has no SKILL.md body; use existing per-skill `download_archive`, extract `SKILL.md` from the zip (`origin: hubArchive`). This downloads one skill package, not an entire hub or git repo.
4. **Frontmatter**: strip leading `---` … `---` block. Header uses parsed `name` / `description` when present, otherwise falls back to list metadata / directory name. Body passed to the renderer excludes the YAML block.
5. **Invalid skills**: still attempt to read the file when a path exists; header may show validation hints from existing skill state.

### Errors

| Case | Behavior |
|------|----------|
| Missing file | Error + retry |
| Network / Hub failure | Error + retry |
| Empty or non-UTF-8 content | Clear error message |
| User closes during load | Ignore late result (generation guard) |

## Frontend Integration

- Introduce `react-markdown` and `remark-gfm` as dependencies for read-only preview only (no editor chrome).
- Own preview open state at a level that both TargetDetail and SkillHubPage can trigger (App-level or a small shared hook), passing `SkillMarkdownRequest`.
- Drawer owns fetch lifecycle for the active request; parent only supplies identity + close.

## Testing

Backend:

- Installed skill returns stripped body and header fields.
- Git discover hits repo cache without network when file exists.
- Git discover missing cache uses single-file fetch path (mocked).
- Hub discover extracts `SKILL.md` from mocked archive.
- Malformed / missing file maps to typed errors.

Frontend:

- Name click opens drawer; checkbox / multi-select / action buttons do not.
- Escape, overlay, and close dismiss the drawer.
- Switching skills reloads content without unmounting the shell incorrectly.
- Skeleton while pending; retry after failure.
- Markdown renders headings/lists/code with GFM basics.
- Opening preview closes source-manage drawer if open (single-drawer rule).

Verification: focused Vitest + Rust tests, full suites, production build.

## Non-Goals Recap

No editing, no raw/source toggle, no TOC dual-pane, no whole-repo redownload for Git preview, no new Hub protocol beyond existing archive download for Hub discover preview.
