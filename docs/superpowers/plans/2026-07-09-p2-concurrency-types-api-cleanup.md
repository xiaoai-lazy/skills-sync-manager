# P2 Concurrency / Types / API Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix startup discover/updates race visibility, align AppConfig TS fields with a drift check, and remove unused `skillsDir` from `updateTarget`.

**Architecture:** Three sequential commits on `feat/v0.6`. (1) Frontend serializes startup discover→checkUpdates and treats InProgress as silent on startup / light prompt on manual refresh. (2) Add `gitlabCredentialHosts` plus a Vitest key-set alignment test against `models.rs` AppConfig. (3) Narrow `update_target` / `updateTarget` to name-only; document LinkStrategy as auto-only.

**Tech Stack:** Tauri 2 / Rust, React + Vitest, existing hooks (`useAppBootstrap`, `useSkillHub`, `SkillHubPage`).

**Spec:** `docs/superpowers/specs/2026-07-09-p2-concurrency-types-api-cleanup-design.md`

---

## File map

| File | Phase | Role |
|------|-------|------|
| `src/utils/ipcError.ts` | 1 | Detect `discoverInProgress` / `updatesInProgress` |
| `src/hooks/useAppBootstrap.ts` | 1 | Serial startup; silence InProgress |
| `src/hooks/useSkillHub.ts` | 1 | Silence InProgress in background runners |
| `src/components/skill-hub/SkillHubPage.tsx` | 1 | Manual refresh busy feedback |
| `src/test/ipcError.test.ts` | 1 | Unit tests for classifier |
| `src/model/types.ts` | 2 | `gitlabCredentialHosts` |
| `src/test/appConfigFieldsAlign.test.ts` | 2 | Drift check |
| `src-tauri/src/commands/mod.rs` | 3 | Drop `skills_dir` param |
| `src-tauri/src/target_registry.rs` | 3 | Narrow update request if needed |
| `src/api/commands.ts` | 3 | `updateTarget(id, name)` |
| `src/hooks/useTargetActions.ts` | 3 | Call site |
| `src/App.tsx` / dialogs / tests | 3 | Stop passing skillsDir |
| `README.md` / `README.zh.md` | 3 | LinkStrategy note |

---

### Task 1: P2-3 — InProgress helper + serial startup

**Files:**
- Create: `src/utils/ipcError.ts`
- Create: `src/test/ipcError.test.ts`
- Modify: `src/hooks/useAppBootstrap.ts`
- Modify: `src/hooks/useSkillHub.ts`
- Modify: `src/components/skill-hub/SkillHubPage.tsx` (if manual path needs toast)

- [ ] **Step 1: Add classifier + tests**

```typescript
// ipcError.ts
export function isInProgressError(err: unknown): boolean {
  const code = /* extract from AppErrorDto-shaped object */ '';
  return code === 'discoverInProgress' || code === 'updatesInProgress';
}
```

- [ ] **Step 2: Serial startup in useAppBootstrap**

Replace `Promise.all([...])` with:

```typescript
void (async () => {
  await runBackgroundDiscover();
  await runBackgroundCheckUpdates();
})();
```

- [ ] **Step 3: Silence InProgress in useSkillHub background runners**

In catch: if `isInProgressError(err)` return; else `setError(...)`.

- [ ] **Step 4: Manual Hub refresh — busy feedback**

When local in-flight or InProgress: `onToast?.('正在刷新，请稍候')` (or equivalent); do not use failure-toned error for InProgress.

- [ ] **Step 5: Verify + commit**

```bash
npx vitest run src/test/ipcError.test.ts src/test/app.test.tsx src/test/SkillHubPage.test.tsx --exclude ".worktrees/**"
git add src/utils/ipcError.ts src/test/ipcError.test.ts src/hooks/useAppBootstrap.ts src/hooks/useSkillHub.ts src/components/skill-hub/SkillHubPage.tsx
git commit -m "fix(p2): serialize startup discover/updates and soften InProgress UX"
```

---

### Task 2: P2-1 — AppConfig field align + drift test

**Files:**
- Modify: `src/model/types.ts`
- Create: `src/test/appConfigFieldsAlign.test.ts`

- [ ] **Step 1: Add `gitlabCredentialHosts?: string[]` to AppConfig**

- [ ] **Step 2: Drift test**

Parse `struct AppConfig` field names from `src-tauri/src/models.rs` (snake_case → camelCase) and compare to TypeScript `AppConfig` interface keys. Maintain `IGNORE_RUST_FIELDS` / `IGNORE_TS_FIELDS` if needed for intentional asymmetries.

- [ ] **Step 3: Verify + commit**

```bash
npx vitest run src/test/appConfigFieldsAlign.test.ts --exclude ".worktrees/**"
git add src/model/types.ts src/test/appConfigFieldsAlign.test.ts
git commit -m "feat(p2): align AppConfig TS fields and add drift check"
```

---

### Task 3: P2-4 — updateTarget name-only + LinkStrategy note

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`, `target_registry.rs` (as needed)
- Modify: `src/api/commands.ts`, `src/hooks/useTargetActions.ts`, call sites/tests
- Modify: `README.md`, `README.zh.md`

- [ ] **Step 1: Backend — drop skills_dir from update_target command**

Keep name-only update behavior; update Rust tests that asserted ignore-skills-dir.

- [ ] **Step 2: Frontend — `updateTarget(targetId, name)`**

Update hooks, App dialog confirm, and tests (`toHaveBeenCalledWith('target_1', 'Updated Target')`).

- [ ] **Step 3: README — LinkStrategy currently only `auto`**

- [ ] **Step 4: Verify + commit**

```bash
npx vitest run src/test --exclude ".worktrees/**"
cargo test --manifest-path src-tauri/Cargo.toml
git add -A
git commit -m "refactor(p2): drop unused skillsDir from updateTarget; note LinkStrategy auto-only"
```

---

### Task 4: Full regression

- [ ] **Step 1: Run full suite**

```bash
npx vitest run src/test --exclude ".worktrees/**"
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: Confirm no leftover `updateTarget(..., skillsDir)` or startup `Promise.all` discover+updates**

```bash
rg "Promise\.all\(\[runBackgroundDiscover|updateTarget\([^)]+," src
```

---

## Self-review

- Spec P2-3/1/4 covered; P2-2/5/6/7 out of scope  
- No TBD placeholders  
- Three commits match three phases  
