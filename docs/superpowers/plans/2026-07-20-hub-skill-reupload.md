# Skill Hub 主库重传 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On Skill Hub「已安装」cards, show「已修改」when main-library content diverges from `contentHash`, offer「重新上传」only then (confirm overwrite remote), and confirm「更新」only when locally dirty.

**Architecture:** Reuse `upload_skill_to_hub` for push; after success persist refreshed `skill_records.content_hash`. Annotate `SkillView.localDirty` during `build_skill_hub_local_state` by comparing current main-library hash to `contentHash`. Frontend gates card UI + `ConfirmDialog` branches in `SkillHubPage`.

**Tech Stack:** Rust (Tauri commands), React 18, Vitest + Testing Library, existing `ConfirmDialog` / `uploadSkillToHub` / `updateSkill`.

**Spec:** `docs/superpowers/specs/2026-07-20-hub-skill-reupload-design.md`

## Global Constraints

- Reupload only for installed Hub skills (`source === "skillhub"`) with `localDirty === true`.
- Reupload always confirms覆盖远程; target is original `hubEndpointId` / `hubSkillGroup` / `hubSkillId` (via existing record + `storageKey`).
- Update confirms覆盖本地 only when `localDirty`; otherwise pull immediately.
- Upload success must refresh `contentHash` and clear dirty; failure must not change `contentHash`.
- No in-app editor; no target-dir sync; no Git/local one-click reupload.
- Chinese copy: badge `已修改`; button `重新上传`; toasts on success/error.
- Prefer bundling `localDirty` into scan/hub state (no per-card IPC).

---

## File Structure

| File | Responsibility |
|------|----------------|
| Modify: `src-tauri/src/models.rs` | Add `SkillView.local_dirty` (`#[serde(default)]`) |
| Modify: `src-tauri/src/skill_hub_upload.rs` | After successful upload, update matching `skill_records.content_hash` |
| Modify: `src-tauri/src/commands/skill_hub.rs` | Persist `skill_records` after upload; annotate `localDirty` in `build_skill_hub_local_state` |
| Modify: `src-tauri/src/skill_updates.rs` | Export/reuse hash helper for contentHash-length heuristic (optional small helper) |
| Modify: `src/model/types.ts` | `SkillView.localDirty`; extend `emptyV6SkillViewFields` |
| Modify: `src/components/skill-hub/SkillCard.tsx` | Badge + reupload button gated on `localDirty` |
| Modify: `src/styles/hub.css` | `.badge-dirty` |
| Create: `src/test/SkillCard.test.tsx` | Card visibility for dirty / update / non-hub |
| Modify: `src/components/skill-hub/SkillHubPage.tsx` | Confirm dialogs; reupload handler; conditional update confirm |
| Modify: `src/test/SkillHubPage.test.tsx` | Integration tests for confirm branches + upload invoke |

---

### Task 1: Persist `contentHash` after Hub upload (Rust)

**Files:**
- Modify: `src-tauri/src/skill_hub_upload.rs`
- Modify: `src-tauri/src/commands/skill_hub.rs` (persist `skill_records` from upload result, not only discover cache)
- Test: unit tests in `src-tauri/src/skill_hub_upload.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: existing `upload_skill_to_hub(config, hub_endpoint_id, group, storage_key, main_dir)`
- Produces: on success, `config.skill_records[key].content_hash` equals hash of uploaded main-library dir; command persists that record change

- [ ] **Step 1: Write the failing test**

In `skill_hub_upload.rs` tests (add module if missing), cover: given a hub `SkillRecord` with stale `content_hash`, after a successful upload path that updates hash (extract a testable helper if HTTP client is hard to mock):

```rust
#[test]
fn refresh_content_hash_after_upload_updates_matching_record() {
    // Arrange: temp main dir + SKILL.md, skill_records entry with content_hash = "stale"
    // Act: call helper `refresh_record_content_hash_after_upload(config, storage_key, &skill_dir)`
    // Assert: record.content_hash == compute_skill_md_hash_prefix(&skill_dir) (for skillhub)
    //         and != "stale"
}
```

Prefer extracting:

```rust
pub(crate) fn refresh_record_content_hash_after_upload(
    config: &mut AppConfig,
    storage_key: &str,
    skill_dir: &Path,
) -> Result<(), AppError>
```

so the test does not need a live Hub HTTP server.

- [ ] **Step 2: Run test to verify it fails**

Run (from `src-tauri`):

```bash
cargo test refresh_content_hash_after_upload_updates_matching_record -- --nocapture
```

Expected: FAIL (helper missing or hash not updated).

- [ ] **Step 3: Implement hash refresh + wire into upload**

```rust
pub(crate) fn refresh_record_content_hash_after_upload(
    config: &mut AppConfig,
    storage_key: &str,
    skill_dir: &Path,
) -> Result<(), AppError> {
    let Some(record) = config
        .skill_records
        .get_mut(storage_key)
        .or_else(|| {
            // fallback: find by record.storage_key == storage_key
            None
        })
    else {
        return Ok(());
    };

    // Hub installs/updates typically store SKILL.md prefix hashes (12 hex).
    // Match existing update_hub_skill behavior for skillhub.
    record.content_hash = if record.source == "skillhub"
        || (record.content_hash.len() == 12
            && record
                .content_hash
                .chars()
                .all(|c| c.is_ascii_hexdigit()))
    {
        crate::skill_updates::compute_skill_md_hash_prefix(skill_dir)?
    } else {
        crate::skill_updates::compute_dir_hash(skill_dir)?
    };
    Ok(())
}
```

Call it in `upload_skill_to_hub` **after** successful `upload_skill` and before returning.

In `commands/skill_hub.rs::upload_skill_to_hub`, when merging into `latest`, also copy updated records:

```rust
latest.skill_discover_cache = config.skill_discover_cache;
latest.skill_records = config.skill_records; // upload only mutates content_hash for one key; records otherwise unchanged in this task
```

Keep the existing comment intent: do not clobber `skill_update_cache` from the stale `config` snapshot — only assign discover cache + skill_records from the upload task’s config.

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test refresh_content_hash_after_upload_updates_matching_record -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/skill_hub_upload.rs src-tauri/src/commands/skill_hub.rs
git commit -m "feat: refresh skill contentHash after Hub upload"
```

---

### Task 2: Annotate `SkillView.localDirty` in hub local state (Rust)

**Files:**
- Modify: `src-tauri/src/models.rs` (`SkillView`)
- Modify: `src-tauri/src/commands/skill_hub.rs` (`build_skill_hub_local_state`)
- Modify: `src-tauri/src/skill_library.rs` only if constructors need updating (prefer `#[serde(default)]` + `Default`)
- Test: `src-tauri/src/commands/skill_hub.rs` tests

**Interfaces:**
- Consumes: `SkillRecord.content_hash`, `SkillRecord.source`, skill path on disk
- Produces: `SkillView.local_dirty: bool` (serde `localDirty`)

- [ ] **Step 1: Write the failing test**

In `commands/skill_hub.rs` tests:

```rust
#[test]
fn build_skill_hub_local_state_marks_local_dirty_when_hash_diverges() {
    // main dir with hub/{endpoint}/{group}/{id}/SKILL.md
    // skill_records[storage_key] = skillhub record with content_hash != current hash
    let state = build_skill_hub_local_state(&main_dir, &config).unwrap();
    let skill = state.skills.iter().find(|s| s.storage_key == storage_key).unwrap();
    assert!(skill.local_dirty);
}

#[test]
fn build_skill_hub_local_state_clears_local_dirty_when_hash_matches() {
    // content_hash == compute_skill_md_hash_prefix(skill_dir)
    assert!(!skill.local_dirty);
}

#[test]
fn build_skill_hub_local_state_non_hub_is_not_local_dirty() {
    // github/gitlab/local record → local_dirty false even if hashes differ
    assert!(!skill.local_dirty);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test build_skill_hub_local_state_marks_local_dirty -- --nocapture
```

Expected: FAIL (`local_dirty` field missing).

- [ ] **Step 3: Implement field + annotation**

In `models.rs`:

```rust
pub struct SkillView {
    // ...existing fields...
    #[serde(default)]
    pub local_dirty: bool,
}
```

Update `Default` to `local_dirty: false`.

Add helper (in `skill_hub.rs` or `skill_updates.rs`):

```rust
fn annotate_local_dirty(skills: &mut [SkillView], config: &AppConfig) {
    for skill in skills.iter_mut() {
        skill.local_dirty = false;
        if skill.storage_key.is_empty() || !skill.path.is_dir() {
            continue;
        }
        let Some(record) = config.skill_records.get(&skill.storage_key).or_else(|| {
            config
                .skill_records
                .values()
                .find(|r| r.storage_key == skill.storage_key)
        }) else {
            continue;
        };
        if record.source != "skillhub" || record.content_hash.is_empty() {
            continue;
        }
        let Ok(current) = hash_matching_stored_content_hash(&skill.path, &record.content_hash) else {
            continue;
        };
        skill.local_dirty = current != record.content_hash;
    }
}

fn hash_matching_stored_content_hash(path: &Path, content_hash: &str) -> Result<String, AppError> {
    if content_hash.len() == 12 && content_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        skill_updates::compute_skill_md_hash_prefix(path)
    } else {
        skill_updates::compute_dir_hash(path)
    }
}
```

Call `annotate_local_dirty` at end of `build_skill_hub_local_state` before returning.

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test build_skill_hub_local_state_marks_local_dirty -- --nocapture
cargo test build_skill_hub_local_state_clears_local_dirty -- --nocapture
cargo test build_skill_hub_local_state_non_hub_is_not_local_dirty -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands/skill_hub.rs src-tauri/src/skill_updates.rs
git commit -m "feat: expose localDirty on SkillView for Hub skills"
```

---

### Task 3: TypeScript `SkillView.localDirty` + SkillCard UI

**Files:**
- Modify: `src/model/types.ts`
- Modify: `src/components/skill-hub/SkillCard.tsx`
- Modify: `src/styles/hub.css`
- Create: `src/test/SkillCard.test.tsx`

**Interfaces:**
- Consumes: `SkillView.localDirty` from scan state
- Produces:
  ```ts
  // SkillCardProps additions
  onReupload?: () => void;
  // installed mode: show badge「已修改」when localDirty
  // show「重新上传」only when mode==='installed' && localDirty && !invalid && onReupload
  ```

- [ ] **Step 1: Write the failing tests**

Create `src/test/SkillCard.test.tsx`:

```tsx
import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import SkillCard from '../components/skill-hub/SkillCard';
import { emptyV6SkillViewFields } from '../model/types';
import type { SkillView } from '../model/types';

afterEach(() => cleanup());

function hubSkill(overrides: Partial<SkillView> = {}): SkillView {
  return {
    ...emptyV6SkillViewFields,
    dirName: 'tdd',
    name: 'tdd',
    description: 'desc',
    path: 'C:/skills/hub/e/g/tdd',
    valid: true,
    validationErrors: [],
    storageKey: 'hub/e/g/tdd',
    linkName: 'tdd',
    localDirty: false,
    ...overrides,
  };
}

describe('SkillCard reupload', () => {
  it('shows 已修改 and 重新上传 when localDirty', async () => {
    const onReupload = vi.fn();
    render(
      <SkillCard
        skill={hubSkill({ localDirty: true })}
        mode="installed"
        sourceLabel="Skill Hub · g"
        onReupload={onReupload}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText('已修改')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '重新上传' }));
    expect(onReupload).toHaveBeenCalledTimes(1);
  });

  it('hides 重新上传 when not localDirty', () => {
    render(
      <SkillCard
        skill={hubSkill({ localDirty: false })}
        mode="installed"
        sourceLabel="Skill Hub · g"
        onReupload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.queryByText('已修改')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '重新上传' })).not.toBeInTheDocument();
  });

  it('shows 更新 and 重新上传 together when hasUpdate and localDirty', () => {
    render(
      <SkillCard
        skill={hubSkill({ localDirty: true })}
        mode="installed"
        hasUpdate
        sourceLabel="Skill Hub · g"
        onUpdate={vi.fn()}
        onReupload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: '更新' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重新上传' })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
npm test -- src/test/SkillCard.test.tsx
```

Expected: FAIL (`localDirty` / `onReupload` not implemented).

- [ ] **Step 3: Implement types, card, CSS**

`types.ts`:

```ts
export interface SkillView {
  // ...existing...
  /** Hub skill: main-library hash differs from record.contentHash */
  localDirty?: boolean;
}

export const emptyV6SkillViewFields = {
  storageKey: '',
  linkName: '',
  localDirty: false,
} as const;
```

`SkillCard.tsx` (installed branch):

- Read `localDirty` from `SkillView` when `!isDiscoverableSkill(skill)`: `Boolean(skill.localDirty)`.
- Include badge `已修改` in `showBadges` when `localDirty`.
- Actions order: `[更新?][重新上传?][删除]` — reupload button `className="btn-sm"` with soft style if available, only when `localDirty && onReupload && !invalid`.

`hub.css`:

```css
.badge-dirty {
  background: var(--warning-soft);
  color: #92400e;
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
npm test -- src/test/SkillCard.test.tsx
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/model/types.ts src/components/skill-hub/SkillCard.tsx src/styles/hub.css src/test/SkillCard.test.tsx
git commit -m "feat: show dirty badge and reupload on SkillCard"
```

---

### Task 4: Wire SkillHubPage confirm + reupload + conditional update

**Files:**
- Modify: `src/components/skill-hub/SkillHubPage.tsx`
- Modify: `src/test/SkillHubPage.test.tsx`

**Interfaces:**
- Consumes: `uploadSkillToHub`, `updateSkill`, `ConfirmDialog`, `resolveSkillRecord`, `skill.localDirty`
- Produces: page-level confirm state for `reupload` | `update-overwrite-local`

- [ ] **Step 1: Write the failing tests**

Extend `SkillHubPage.test.tsx` (mock `uploadSkillToHub` / `updateSkill` like existing API mocks):

```tsx
it('opens remote-overwrite confirm when clicking 重新上传 on dirty hub skill', async () => {
  // render page with installed hub skill localDirty:true + skillRecords hub meta
  // click 重新上传 → expect dialog title/message mentioning 覆盖 / 远程
  // confirm → expect uploadSkillToHub(endpointId, group, storageKey)
  // expect toast success path (onToast)
});

it('skips confirm and updates immediately when not localDirty', async () => {
  // hasUpdate + localDirty false
  // click 更新 → updateSkill called without ConfirmDialog
});

it('confirms before update when localDirty', async () => {
  // hasUpdate + localDirty true
  // click 更新 → dialog 覆盖本地 → confirm → updateSkill
});

it('does not upload when reupload confirm is cancelled', async () => {
  // click 重新上传 → 取消 → uploadSkillToHub not called
});
```

Use exact storageKey / hubEndpointId / hubSkillGroup from fixtures matching `resolveSkillRecord`.

- [ ] **Step 2: Run tests to verify they fail**

```bash
npm test -- src/test/SkillHubPage.test.tsx
```

Expected: FAIL on new cases.

- [ ] **Step 3: Implement page wiring**

In `SkillHubPage.tsx`:

1. Import `ConfirmDialog` and `uploadSkillToHub`.
2. State:
   ```ts
   type PendingConfirm =
     | { kind: 'reupload'; skill: SkillView }
     | { kind: 'update-local'; skill: SkillView }
     | null;
   const [pendingConfirm, setPendingConfirm] = useState<PendingConfirm>(null);
   const [confirmBusy, setConfirmBusy] = useState(false);
   ```
3. Replace direct `onUpdate={() => void handleUpdateSkill(skill)}` with:
   ```ts
   onUpdate={() => {
     if (skill.localDirty) setPendingConfirm({ kind: 'update-local', skill });
     else void handleUpdateSkill(skill);
   }}
   onReupload={() => setPendingConfirm({ kind: 'reupload', skill })}
   ```
   Only pass `onReupload` when `skill.localDirty` (or always pass; card already gates — prefer always pass handler, card gates visibility).
4. `handleReuploadSkill`:
   ```ts
   const record = resolveSkillRecord(skill, installedRecords);
   if (!record?.hubEndpointId || !record.hubSkillGroup) {
     onError?.('无法重新上传：缺少 Hub 来源信息');
     return;
   }
   await uploadSkillToHub(record.hubEndpointId, record.hubSkillGroup, skill.storageKey);
   // optional: onDiscoverSkillsChange from result.discoverSkills if page owns discover list
   await checkSkillUpdates?.() / onPendingUpdatesChange via existing refresh patterns
   await refreshHubState();
   onToast?.('已重新上传到 Hub');
   ```
5. ConfirmDialog:
   - reupload: title `重新上传到 Hub？`; message includes endpoint/group/id + 会覆盖远程; confirmLabel `确认覆盖远程`
   - update-local: title `从 Hub 更新到本地？`; message 会覆盖本地; confirmLabel `确认覆盖本地`
6. On confirm: set busy, run action, clear pending, handle errors with `onError`.

- [ ] **Step 4: Run tests to verify they pass**

```bash
npm test -- src/test/SkillHubPage.test.tsx
npm test -- src/test/SkillCard.test.tsx
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/skill-hub/SkillHubPage.tsx src/test/SkillHubPage.test.tsx
git commit -m "feat: confirm Hub reupload and dirty-aware update"
```

---

### Task 5: Regression sweep + fixture cleanup

**Files:**
- Modify any TS fixtures still missing `localDirty` if tests break
- Modify Rust constructors only if compile errors remain (prefer `Default`)

- [ ] **Step 1: Run full frontend tests**

```bash
npm test
```

Expected: PASS (fix any `SkillView` object literals missing fields if TypeScript requires `localDirty` — keep it optional `localDirty?: boolean` to minimize churn).

- [ ] **Step 2: Run focused Rust tests**

```bash
cargo test build_skill_hub_local_state -- --nocapture
cargo test refresh_content_hash_after_upload -- --nocapture
```

Expected: PASS

- [ ] **Step 3: Commit only if fixes were needed**

```bash
git add -A
git commit -m "test: fix fixtures after localDirty field"
```

(Skip empty commit if nothing changed.)

---

## Spec Coverage Checklist

| Spec requirement | Task |
|------------------|------|
| Hub-only dirty detection via contentHash | Task 2 |
| Badge「已修改」+ reupload only when dirty | Task 3 |
| Confirm overwrite remote then upload | Task 4 |
| Upload refreshes contentHash | Task 1 |
| Update confirm only when dirty | Task 4 |
| Toast / error without hash write on failure | Task 1 + 4 |
| Out of scope (editor, target sync, git reupload) | N/A (not implemented) |

## Self-Review Notes

- No new Hub HTTP API; reuse `uploadSkillToHub`.
- Command persist path must update `skill_records` without clobbering `skill_update_cache`.
- Hash algorithm uses stored `contentHash` length heuristic (12 hex → SKILL.md prefix) to match install/update history.
