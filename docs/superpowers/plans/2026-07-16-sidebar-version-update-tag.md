# Sidebar Version + Update Tag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the app version in the sidebar footer, surface updates as a clickable tag (never auto-open the dialog), and allow manual async re-check via a refresh icon with in-flight + generation race guards.

**Architecture:** Extract update-check logic into `useAppUpdater` (shared startup + manual path). Sidebar renders a pinned footer: `v{version}` plus either an update tag or a refresh button. `App.tsx` wires version fetch, check results, dialog open-from-tag, and toasts. Rust `check_app_update` stays unchanged (async metadata-only).

**Tech Stack:** React 18, Vitest + Testing Library, Tauri 2 (`@tauri-apps/api/app` `getVersion`, existing `check_app_update` invoke).

**Spec:** `docs/superpowers/specs/2026-07-16-sidebar-version-update-tag-design.md`

## Global Constraints

- Never auto-open `UpdateDialog` after a successful check (startup or manual).
- Update check must remain async (`await checkAppUpdate()`); no sync/blocking wrappers.
- Shared in-flight gate: while checking, ignore further clicks; disable refresh control.
- Generation guard: only apply results from the latest started check.
- 「稍后」keeps `updateInfo` (tag stays); dialog can reopen via tag.
- Refresh icon only when `updateInfo === null`; tag only when `updateInfo !== null`.
- No silent download; install only via existing「立即更新」path.
- Chinese UI copy: tag `有新版本`; refresh `aria-label` `检查更新`; toasts `已是最新` / `检查更新失败`.

---

## File Structure

| File | Responsibility |
|------|----------------|
| Create: `src/hooks/useAppUpdater.ts` | Version state, updateInfo, dialog open, checking, `runUpdateCheck`, defer/install |
| Create: `src/test/useAppUpdater.test.ts` | Race / in-flight / no-auto-open / defer-keeps-info unit tests |
| Modify: `src/components/Sidebar.tsx` | Footer UI: version + tag or refresh |
| Modify: `src/styles/shell.css` | Footer / tag / refresh / busy styles |
| Modify: `src/test/Sidebar.test.tsx` | Footer rendering + click callbacks |
| Modify: `src/hooks/useAppDialogs.ts` | Remove update-related state (moved to hook) |
| Modify: `src/App.tsx` | Use `useAppUpdater`; pass Sidebar props; toast for manual check |
| Modify: `src/test/UpdateDialog.test.tsx` | Replace old auto-open startup harness with new expectations |
| Modify: `src/test/app.test.tsx` | Mock `getVersion` if needed; ensure startup does not open dialog |

---

### Task 1: `useAppUpdater` hook (async + race guards)

**Files:**
- Create: `src/hooks/useAppUpdater.ts`
- Create: `src/test/useAppUpdater.test.ts`
- Modify: `src/hooks/useAppDialogs.ts` (remove update fields after App migrates in Task 3 — do the removal in Task 3; this task only adds the new hook)

**Interfaces:**
- Consumes: `checkAppUpdate`, `installAppUpdate` from `../api/updater`; `getVersion` from `@tauri-apps/api/app`; `errorMessage` from `../utils/errorMessage`
- Produces:
  ```ts
  export type UpdateCheckSource = 'startup' | 'manual';

  export type UseAppUpdaterResult = {
    appVersion: string | null;
    updateInfo: UpdateInfo | null;
    updateDialogOpen: boolean;
    updateInstalling: boolean;
    updateError: string | null;
    updateChecking: boolean;
    runUpdateCheck: (source: UpdateCheckSource) => Promise<void>;
    openUpdateDialog: () => void;
    handleDeferUpdate: () => void;
    handleInstallUpdate: () => Promise<void>;
  };

  export function useAppUpdater(args: {
    enabled: boolean; // true when appState is loaded
    onToast: (message: string, kind: 'success' | 'error') => void;
  }): UseAppUpdaterResult;
  ```

- [ ] **Step 1: Write the failing tests**

Create `src/test/useAppUpdater.test.ts`:

```ts
import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useAppUpdater } from '../hooks/useAppUpdater';

vi.mock('../api/updater', () => ({
  checkAppUpdate: vi.fn(),
  installAppUpdate: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(),
}));

import { checkAppUpdate, installAppUpdate } from '../api/updater';
import { getVersion } from '@tauri-apps/api/app';

const sampleUpdate = {
  version: '0.8.0',
  currentVersion: '0.7.1',
  notes: 'notes',
};

describe('useAppUpdater', () => {
  beforeEach(() => {
    vi.mocked(getVersion).mockResolvedValue('0.7.1');
    vi.mocked(checkAppUpdate).mockResolvedValue(null);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads app version when enabled', async () => {
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.appVersion).toBe('0.7.1');
    });
  });

  it('startup check sets updateInfo but does not open dialog', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.updateInfo?.version).toBe('0.8.0');
    });
    expect(result.current.updateDialogOpen).toBe(false);
    expect(onToast).not.toHaveBeenCalled();
  });

  it('ignores a second check while in flight', async () => {
    let resolveCheck!: (v: typeof sampleUpdate | null) => void;
    vi.mocked(checkAppUpdate).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve;
        })
    );
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    await act(async () => {
      const p1 = result.current.runUpdateCheck('manual');
      const p2 = result.current.runUpdateCheck('manual');
      await Promise.resolve();
      expect(result.current.updateChecking).toBe(true);
      resolveCheck(null);
      await p1;
      await p2;
    });

    expect(checkAppUpdate).toHaveBeenCalledTimes(1);
  });

  it('applies only the latest generation result', async () => {
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    let resolveFirst!: (v: typeof sampleUpdate | null) => void;
    let resolveSecond!: (v: typeof sampleUpdate | null) => void;

    vi.mocked(checkAppUpdate)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve as (v: typeof sampleUpdate | null) => void;
          })
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveSecond = resolve as (v: typeof sampleUpdate | null) => void;
          })
      );

    // Bypass in-flight by finishing first gate carefully:
    // Start first, force-clear inFlight only if implementation allows sequential
    // Prefer: start check A, wait until in flight, then we need a way to start B.
    // Spec: in-flight blocks B. Generation still matters if inFlight is cleared
    // between checks. Test sequential: A starts and completes after B already completed.

    await act(async () => {
      const pA = result.current.runUpdateCheck('manual');
      await Promise.resolve();
      // Still in flight — second call ignored
      await result.current.runUpdateCheck('manual');
      resolveFirst(sampleUpdate);
      await pA;
    });
    expect(result.current.updateInfo?.version).toBe('0.8.0');

    // Now start a newer check that returns null; ensure it wins
    vi.mocked(checkAppUpdate).mockResolvedValueOnce(null);
    await act(async () => {
      await result.current.runUpdateCheck('manual');
    });
    expect(result.current.updateInfo).toBeNull();
    expect(onToast).toHaveBeenCalledWith('已是最新', 'success');
  });

  it('manual failure toasts and keeps updateInfo null', async () => {
    vi.mocked(checkAppUpdate).mockRejectedValue(new Error('network'));
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    await act(async () => {
      await result.current.runUpdateCheck('manual');
    });

    expect(result.current.updateInfo).toBeNull();
    expect(onToast).toHaveBeenCalledWith('检查更新失败', 'error');
  });

  it('openUpdateDialog opens; defer closes but keeps updateInfo', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.updateInfo).not.toBeNull();
    });

    act(() => {
      result.current.openUpdateDialog();
    });
    expect(result.current.updateDialogOpen).toBe(true);

    act(() => {
      result.current.handleDeferUpdate();
    });
    expect(result.current.updateDialogOpen).toBe(false);
    expect(result.current.updateInfo?.version).toBe('0.8.0');
  });
});
```

Simplify the “generation” test if the double-promise setup is awkward: keep the sequential newer-wins test above (null after sampleUpdate). That still validates applying the latest completed check. Optionally add a true stale-out-of-order test only if the hook exposes a test seam; preferred implementation uses generation so out-of-order can be tested by temporarily clearing `inFlight` between starts — **implement generation regardless**; for the unit test, sequential newer-wins + in-flight ignore is sufficient coverage.

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- src/test/useAppUpdater.test.ts`

Expected: FAIL (module / hook not found)

- [ ] **Step 3: Implement `useAppUpdater`**

Create `src/hooks/useAppUpdater.ts`:

```ts
import { useCallback, useEffect, useRef, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import {
  checkAppUpdate,
  installAppUpdate,
  type UpdateInfo,
} from '../api/updater';
import { errorMessage } from '../utils/errorMessage';

export type UpdateCheckSource = 'startup' | 'manual';

export type UseAppUpdaterResult = {
  appVersion: string | null;
  updateInfo: UpdateInfo | null;
  updateDialogOpen: boolean;
  updateInstalling: boolean;
  updateError: string | null;
  updateChecking: boolean;
  runUpdateCheck: (source: UpdateCheckSource) => Promise<void>;
  openUpdateDialog: () => void;
  handleDeferUpdate: () => void;
  handleInstallUpdate: () => Promise<void>;
};

export function useAppUpdater(args: {
  enabled: boolean;
  onToast: (message: string, kind: 'success' | 'error') => void;
}): UseAppUpdaterResult {
  const { enabled, onToast } = args;
  const onToastRef = useRef(onToast);
  onToastRef.current = onToast;

  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);

  const inFlightRef = useRef(false);
  const generationRef = useRef(0);
  const versionLoadedRef = useRef(false);
  const startupStartedRef = useRef(false);

  const runUpdateCheck = useCallback(async (source: UpdateCheckSource) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    const generation = ++generationRef.current;
    setUpdateChecking(true);
    try {
      const info = await checkAppUpdate();
      if (generation !== generationRef.current) return;
      setUpdateInfo(info);
      if (source === 'manual') {
        if (info) {
          // tag appears; no dialog
        } else {
          onToastRef.current('已是最新', 'success');
        }
      }
    } catch {
      if (generation !== generationRef.current) return;
      if (source === 'manual') {
        onToastRef.current('检查更新失败', 'error');
      }
      // startup: silent; leave updateInfo unchanged
    } finally {
      if (generation === generationRef.current) {
        inFlightRef.current = false;
        setUpdateChecking(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!enabled || versionLoadedRef.current) return;
    versionLoadedRef.current = true;
    void getVersion()
      .then((v) => setAppVersion(v))
      .catch(() => {
        /* omit version on failure */
      });
  }, [enabled]);

  useEffect(() => {
    if (!enabled || startupStartedRef.current) return;
    startupStartedRef.current = true;
    void runUpdateCheck('startup');
  }, [enabled, runUpdateCheck]);

  const openUpdateDialog = useCallback(() => {
    setUpdateDialogOpen(true);
  }, []);

  const handleDeferUpdate = useCallback(() => {
    setUpdateDialogOpen(false);
    setUpdateError(null);
  }, []);

  const handleInstallUpdate = useCallback(async () => {
    setUpdateInstalling(true);
    setUpdateError(null);
    try {
      await installAppUpdate();
    } catch (err) {
      setUpdateError(errorMessage(err));
      setUpdateInstalling(false);
    }
  }, []);

  return {
    appVersion,
    updateInfo,
    updateDialogOpen,
    updateInstalling,
    updateError,
    updateChecking,
    runUpdateCheck,
    openUpdateDialog,
    handleDeferUpdate,
    handleInstallUpdate,
  };
}
```

**Race note:** In `finally`, only clear `inFlight` / `updateChecking` when `generation === generationRef.current`. If a newer check somehow started, the newer check owns the gate. With the in-flight early-return, a newer check cannot start until the older clears — so also clear `inFlight` unconditionally in `finally` of the check that holds the gate (the early-return means only one holder). Prefer:

```ts
} finally {
  if (generation === generationRef.current) {
    inFlightRef.current = false;
    setUpdateChecking(false);
  }
}
```

Because in-flight blocks overlaps, generation mainly protects against applying a rejected/stale path after a later completed check if the gate is ever relaxed. Keep both.

On manual check that finds an update after prior null: `setUpdateInfo(info)` is enough. On manual check that finds null after prior update: clear tag via `setUpdateInfo(null)` — intentional (user asked to re-check).

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/test/useAppUpdater.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useAppUpdater.ts src/test/useAppUpdater.test.ts
git commit -m "feat: add useAppUpdater with async race-safe checks"
```

---

### Task 2: Sidebar footer UI (version + tag / refresh)

**Files:**
- Modify: `src/components/Sidebar.tsx`
- Modify: `src/styles/shell.css`
- Modify: `src/test/Sidebar.test.tsx`

**Interfaces:**
- Consumes: none from Task 1 types directly (props only)
- Produces Sidebar props extension:
  ```ts
  appVersion?: string | null;
  updateAvailable?: boolean;
  updateChecking?: boolean;
  onOpenUpdate?: () => void;
  onCheckUpdate?: () => void;
  ```

- [ ] **Step 1: Write the failing Sidebar tests**

Extend `src/test/Sidebar.test.tsx` defaults with optional props (undefined OK). Add:

```ts
  it('shows version in the footer', () => {
    renderSidebar({ appVersion: '0.7.1' });
    expect(screen.getByText('v0.7.1')).toBeInTheDocument();
  });

  it('shows update tag when updateAvailable and opens on click', async () => {
    const onOpenUpdate = vi.fn();
    const user = userEvent.setup();
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: true,
      onOpenUpdate,
    });
    expect(screen.queryByRole('button', { name: '检查更新' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '有新版本' }));
    expect(onOpenUpdate).toHaveBeenCalledTimes(1);
  });

  it('shows refresh control when no update and calls onCheckUpdate', async () => {
    const onCheckUpdate = vi.fn();
    const user = userEvent.setup();
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: false,
      updateChecking: false,
      onCheckUpdate,
    });
    await user.click(screen.getByRole('button', { name: '检查更新' }));
    expect(onCheckUpdate).toHaveBeenCalledTimes(1);
  });

  it('disables refresh while updateChecking', () => {
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: false,
      updateChecking: true,
      onCheckUpdate: vi.fn(),
    });
    expect(screen.getByRole('button', { name: '检查更新' })).toBeDisabled();
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test -- src/test/Sidebar.test.tsx`

Expected: FAIL on missing footer / buttons

- [ ] **Step 3: Implement Sidebar footer + CSS**

In `Sidebar.tsx`, extend props and append footer before closing `</aside>`:

```tsx
export interface SidebarProps {
  // ...existing props...
  appVersion?: string | null;
  updateAvailable?: boolean;
  updateChecking?: boolean;
  onOpenUpdate?: () => void;
  onCheckUpdate?: () => void;
}

// inside return, after projects block:
      <div className="sidebar-footer">
        {props.appVersion ? (
          <span className="sidebar-version">v{props.appVersion}</span>
        ) : null}
        {props.updateAvailable ? (
          <button
            type="button"
            className="sidebar-update-tag"
            onClick={props.onOpenUpdate}
          >
            有新版本
          </button>
        ) : (
          <button
            type="button"
            className={`sidebar-update-refresh${props.updateChecking ? ' is-busy' : ''}`}
            aria-label="检查更新"
            disabled={props.updateChecking || !props.onCheckUpdate}
            onClick={props.onCheckUpdate}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M21 12a9 9 0 1 1-2.6-6.4" />
              <path d="M21 3v6h-6" />
            </svg>
          </button>
        )}
      </div>
```

When `appVersion` is null/undefined and no handlers, still render footer only if version or update controls needed. Always render footer row if `appVersion` or update callbacks exist; in App we always pass them.

Add to `shell.css`:

```css
.sidebar-footer {
  margin-top: auto;
  padding: var(--space-2);
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.sidebar-version {
  font-size: 12px;
  color: var(--text-tertiary);
}

.sidebar-update-tag {
  border: none;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  background: var(--accent-soft, var(--surface-muted));
  color: var(--accent);
  box-shadow: none;
}

.sidebar-update-refresh {
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: none;
}

.sidebar-update-refresh:hover:not(:disabled) {
  background: var(--nav-hover);
  color: var(--accent);
}

.sidebar-update-refresh:disabled {
  cursor: default;
  opacity: 0.6;
}

.sidebar-update-refresh.is-busy svg {
  animation: sidebar-refresh-spin 0.8s linear infinite;
}

@keyframes sidebar-refresh-spin {
  to {
    transform: rotate(360deg);
  }
}
```

Ensure `.sidebar` remains `display: flex; flex-direction: column` (already true) so `margin-top: auto` pins the footer.

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/test/Sidebar.test.tsx`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/Sidebar.tsx src/styles/shell.css src/test/Sidebar.test.tsx
git commit -m "feat: show version and update controls in sidebar footer"
```

---

### Task 3: Wire `App.tsx` + clean `useAppDialogs`

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/hooks/useAppDialogs.ts`
- Modify: `src/test/app.test.tsx` (mock `getVersion` / assert no auto dialog)
- Modify: `src/test/UpdateDialog.test.tsx` (replace auto-open startup suite)

**Interfaces:**
- Consumes: `useAppUpdater` from Task 1; Sidebar props from Task 2
- Produces: integrated app behavior matching the spec

- [ ] **Step 1: Update failing / outdated tests first**

In `src/test/UpdateDialog.test.tsx`, **remove** the `StartupUpdateChecker` component and the `describe('Startup update check', ...)` block that expects auto-open. Keep pure `UpdateDialog` unit tests.

In `src/test/app.test.tsx`, add (near other mocks):

```ts
vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.7.1'),
}));
```

Add an integration-style test (or extend an existing mount test):

```ts
  it('shows sidebar version and does not auto-open update dialog when update exists', async () => {
    const { checkAppUpdate } = await import('../api/updater');
    vi.mocked(checkAppUpdate).mockResolvedValue({
      version: '0.8.0',
      currentVersion: '0.7.1',
      notes: 'n',
    });

    render(<App />);

    expect(await screen.findByText('v0.7.1')).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: '有新版本' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
```

Adjust if `App` test harness needs `getAppState` resolved first (follow existing patterns in `app.test.tsx` for waiting on loaded UI).

- [ ] **Step 2: Run to verify new App expectation fails / UpdateDialog suite is green without startup block**

Run: `npm test -- src/test/UpdateDialog.test.tsx src/test/app.test.tsx`

Expected: UpdateDialog PASS; App new test FAIL until wiring

- [ ] **Step 3: Wire App + slim useAppDialogs**

In `useAppDialogs.ts`, delete:

- `updateDismissedRef`, `updateCheckStartedRef`
- `updateDialogOpen`, `setUpdateDialogOpen`
- `updateInfo`, `setUpdateInfo`
- `updateInstalling`, `setUpdateInstalling`
- `updateError`, `setUpdateError`

and their return fields.

In `App.tsx`:

1. Remove imports/usages of those dialog fields for updates.
2. Add:

```ts
const {
  appVersion,
  updateInfo,
  updateDialogOpen,
  updateInstalling,
  updateError,
  updateChecking,
  runUpdateCheck,
  openUpdateDialog,
  handleDeferUpdate,
  handleInstallUpdate,
} = useAppUpdater({
  enabled: Boolean(appState),
  onToast: (message, kind) => setAppToast({ message, kind }),
});
```

3. Delete the old `useEffect` that called `checkAppUpdate` + `setUpdateDialogOpen(true)`, and delete local `handleDeferUpdate` / `handleInstallUpdate` if duplicated.

4. Pass to Sidebar:

```tsx
appVersion={appVersion}
updateAvailable={Boolean(updateInfo)}
updateChecking={updateChecking}
onOpenUpdate={openUpdateDialog}
onCheckUpdate={() => {
  void runUpdateCheck('manual');
}}
```

5. Keep `<UpdateDialog ...>` bound to the hook fields.

- [ ] **Step 4: Run full relevant tests**

Run: `npm test -- src/test/useAppUpdater.test.ts src/test/Sidebar.test.tsx src/test/UpdateDialog.test.tsx src/test/app.test.tsx`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/hooks/useAppDialogs.ts src/test/app.test.tsx src/test/UpdateDialog.test.tsx
git commit -m "feat: wire sidebar update tag and stop auto-open dialog"
```

---

### Task 4: Manual check smoke + self-review against spec

**Files:**
- None new unless a gap appears

- [ ] **Step 1: Spec coverage checklist (manual)**

Confirm each spec item has code:

| Spec item | Where |
|-----------|--------|
| Version in sidebar footer | Task 2–3 |
| Tag when update available | Task 2–3 |
| Refresh when no update | Task 2–3 |
| No auto dialog | Task 1 + 3 |
| Tag opens dialog | Task 2–3 |
| Defer keeps tag | Task 1 |
| Async check only | Task 1 |
| In-flight ignore + disable | Task 1–2 |
| Generation / latest wins | Task 1 |
| Manual toast 已是最新 / 检查更新失败 | Task 1 |
| Install path unchanged | Task 1 |

- [ ] **Step 2: Run full frontend test suite**

Run: `npm test`

Expected: PASS (fix any unrelated failures only if caused by this change)

- [ ] **Step 3: Commit only if Step 1–2 required extra fixes**

```bash
git add -A
git commit -m "fix: align update footer behavior with design spec"
```

(Skip empty commit if nothing changed.)

---

## Self-Review (plan vs spec)

1. **Spec coverage:** Version footer, tag, refresh, no auto-open, defer-reopen, async + in-flight + generation, toasts, install unchanged — all mapped to Tasks 1–3.
2. **Placeholders:** None; concrete code and commands included.
3. **Type consistency:** `UseAppUpdaterResult`, Sidebar props (`updateAvailable`, `updateChecking`, `onOpenUpdate`, `onCheckUpdate`), toast kinds `'success' | 'error'` match `App` `appToast`.
4. **YAGNI:** No About page, no polling, no Rust changes unless tests reveal invoke issues (not expected; `core:default` includes version).
