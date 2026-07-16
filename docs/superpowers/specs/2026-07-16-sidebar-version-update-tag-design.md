# Sidebar Version + Update Tag Design

## Goal

Let users always see the installed app version, discover available updates without an automatic modal interrupting startup, and manually re-check for updates without freezing the UI.

## Current Behavior

- Startup calls `checkAppUpdate()` once after `appState` loads.
- If an update exists, the app sets `updateInfo` and immediately opens `UpdateDialog`.
- There is no version label in the UI.
- App version source of truth: `tauri.conf.json` / installed binary (currently `0.7.1`).
- `check_app_update` is already an async Tauri command (`updater.check().await`); it must stay off the UI thread and must not download the installer.

## Desired Behavior

1. Sidebar footer always shows the current app version as muted text, e.g. `v0.7.1`.
2. When an update is available, show a clickable tag next to the version (e.g. `有新版本`).
3. When **no** update is pending, show a refresh icon next to the version instead of the tag.
4. Do **not** auto-open `UpdateDialog` when an update is detected (startup or manual check).
5. Clicking the tag opens the existing `UpdateDialog`.
6. Clicking「稍后」closes the dialog only; the tag **remains** and can open the dialog again in the same session.
7. Clicking「立即更新」keeps current download/install behavior.
8. Clicking the refresh icon runs a manual update check (same async command as startup).

## UI

- Placement: bottom of `Sidebar`, pinned under the scrollable nav content.
- Layout: one row — `v{version}` + either update tag **or** refresh control (mutually exclusive).
- Version: secondary/muted typography; not interactive.
- Update tag: compact badge/button; keyboard accessible; only when `updateInfo` is non-null.
- Refresh control: icon button with accessible name（如「检查更新」）; only when `updateInfo` is null.
- While a check is in flight: refresh control disabled / shows busy state; ignore further clicks.
- After a manual check with no update: brief non-blocking feedback（如 toast「已是最新」）; restore idle refresh icon.
- Dialog: reuse `UpdateDialog` unchanged except for open trigger.

## Data Flow

```
startup
  → getVersion() → setAppVersion (footer label)
  → runUpdateCheck({ source: 'startup' })
       → null: show refresh icon
       → UpdateInfo: setUpdateInfo, show tag, do NOT open dialog

click refresh → runUpdateCheck({ source: 'manual' })
       → in-flight: no-op (ignore click)
       → null: keep refresh icon + optional「已是最新」toast
       → UpdateInfo: setUpdateInfo, swap to tag, do NOT open dialog

click tag → setUpdateDialogOpen(true)

「稍后」→ setUpdateDialogOpen(false); clear install error;
         keep updateInfo (tag stays); allow reopen

「立即更新」→ installAppUpdate() (existing path)
```

### Version source

- Prefer `@tauri-apps/api/app` `getVersion()`.
- On failure (e.g. non-Tauri test shell): omit the version label rather than showing a wrong hardcoded value.
- Tests mock `getVersion` / updater APIs as needed.

### Async + race management (required)

Update checks must never block the UI thread:

- Frontend always `await checkAppUpdate()` via the existing async invoke; do not introduce sync/blocking wrappers.
- Rust keeps `check_app_update` as `async` using `updater.check().await` only (metadata check, no `download_and_install`).
- Single shared in-flight gate for **all** checks (startup + manual):
  - `updateCheckInFlightRef` (or equivalent): if true, additional clicks / overlapping startup are no-ops.
  - Set true before invoke; clear in `finally`.
- Generation / stale-result guard:
  - Increment `updateCheckGenerationRef` when starting a check; capture `generation` locally.
  - When the promise resolves, apply `setUpdateInfo` / toast **only if** `generation === updateCheckGenerationRef.current`.
  - This prevents an older slow response from overwriting a newer check’s result.
- While in flight: refresh button `disabled` + busy visual; do not queue multiple checks.
- Manual check errors: non-blocking toast（如「检查更新失败」）; do not open dialog; keep refresh icon.
- Startup check errors: remain silent (existing); no toast spam on launch.

`updateCheckStartedRef` alone is insufficient once manual re-check exists. Prefer the in-flight + generation pattern above for both startup and manual paths, sharing one `runUpdateCheck` helper.

`updateDismissedRef` is no longer needed to suppress the dialog after「稍后」, because auto-open is removed. Prefer deleting that dismiss-for-dialog coupling if nothing else depends on it; tag visibility is driven solely by `updateInfo`.

## Components / Files (expected touch points)

| Area | Change |
|------|--------|
| `Sidebar.tsx` | Footer with version + update tag **or** refresh icon; new props |
| `shell.css` (or adjacent styles) | Footer / tag / refresh busy styles |
| `App.tsx` (or small update hook) | Shared `runUpdateCheck`; pass version/update/checking; open dialog from tag; stop auto-open |
| `useAppDialogs.ts` / update handlers | Simplify defer so tag persists; expose checking state if needed |
| `UpdateDialog.tsx` | No behavioral change required |
| Tests | Version/tag/refresh; no auto-open; defer keeps tag; in-flight ignores second click; stale result ignored |

## Non-Goals

- No silent download before the user confirms「立即更新」.
- No full About page.
- No change to Skill Hub startup refresh.
- No automatic periodic polling; only startup once + user-triggered refresh.

## Error Handling

- Startup update check failure: ignore; show refresh icon.
- Manual update check failure: toast; show refresh icon.
- Version fetch failure: hide version text; tag/refresh still work from update state.
- Install failure: show error inside dialog (existing); dialog stays open; tag remains.

## Testing

1. Renders `v{version}` in sidebar footer when version is available.
2. Startup with update available → tag visible, dialog closed, no refresh icon.
3. Startup with no update → refresh icon visible, no tag.
4. Click tag → dialog opens with correct version notes.
5. 「稍后」→ dialog closes, tag still visible; click tag opens dialog again.
6. Click refresh while idle → checking state; second click does nothing until finished.
7. Manual check finds update → tag appears, dialog stays closed.
8. Manual check finds none →「已是最新」feedback; refresh icon returns.
9. Overlapping / stale responses do not clobber newer `updateInfo`.
10. Existing install / error paths still covered.
11. UI remains interactive during check (no sync blocking in the check path).

## Acceptance

- User can read the installed version without leaving the main UI.
- New releases never interrupt with an automatic modal; only the sidebar tag appears.
- With no pending update, user can manually check via the refresh icon without UI freeze or duplicate in-flight checks.
- Existing update dialog remains the confirmation surface for install.
