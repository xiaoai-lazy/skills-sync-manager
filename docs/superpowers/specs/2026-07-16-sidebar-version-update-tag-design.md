# Sidebar Version + Update Tag Design

## Goal

Let users always see the installed app version, and discover available updates without an automatic modal interrupting startup.

## Current Behavior

- Startup calls `checkAppUpdate()` once after `appState` loads.
- If an update exists, the app sets `updateInfo` and immediately opens `UpdateDialog`.
- There is no version label in the UI.
- App version source of truth: `tauri.conf.json` / installed binary (currently `0.7.1`).

## Desired Behavior

1. Sidebar footer always shows the current app version as muted text, e.g. `v0.7.1`.
2. When an update is available, show a clickable tag next to the version (e.g. `有新版本`).
3. Do **not** auto-open `UpdateDialog` when an update is detected.
4. Clicking the tag opens the existing `UpdateDialog`.
5. Clicking「稍后」closes the dialog only; the tag **remains** and can open the dialog again in the same session.
6. Clicking「立即更新」keeps current download/install behavior.

## UI

- Placement: bottom of `Sidebar`, pinned under the scrollable nav content.
- Layout: one row — `v{version}` + optional update tag.
- Version: secondary/muted typography; not interactive.
- Tag: compact badge/button; keyboard accessible; visible only when `updateInfo` is non-null.
- Dialog: reuse `UpdateDialog` (title, notes, defer/install actions) unchanged except for open trigger.

## Data Flow

```
startup
  → getVersion() → setAppVersion (footer label)
  → checkAppUpdate() once
       → null: no tag
       → UpdateInfo: setUpdateInfo, show tag, do NOT open dialog

click tag → setUpdateDialogOpen(true)

「稍后」→ setUpdateDialogOpen(false); clear install error;
         keep updateInfo (tag stays); allow reopen

「立即更新」→ installAppUpdate() (existing path)
```

### Version source

- Prefer `@tauri-apps/api/app` `getVersion()`.
- On failure (e.g. non-Tauri test shell): omit the version label rather than showing a wrong hardcoded value.
- Tests mock `getVersion` / updater APIs as needed.

### Startup check changes

In `App.tsx` (or the hook that owns update state):

- On successful `checkAppUpdate()` with info: `setUpdateInfo(info)` only.
- Remove `setUpdateDialogOpen(true)` from the auto-check path.
- Keep session single-check guard (`updateCheckStartedRef`).
- `updateDismissedRef` is no longer needed to suppress the dialog after「稍后」, because auto-open is removed. Prefer deleting that dismiss-for-dialog coupling if nothing else depends on it; tag visibility is driven solely by `updateInfo`.

## Components / Files (expected touch points)

| Area | Change |
|------|--------|
| `Sidebar.tsx` | Footer with version + optional update tag; new props |
| `shell.css` (or adjacent styles) | Footer / tag styles |
| `App.tsx` | Pass version + update availability; open dialog from tag click; stop auto-open |
| `useAppDialogs.ts` / update handlers | Simplify defer so tag persists |
| `UpdateDialog.tsx` | No behavioral change required |
| Tests | Sidebar version/tag; app startup no longer auto-opens dialog; defer keeps tag |

## Non-Goals

- No periodic / background re-check beyond the existing once-per-session startup check.
- No silent download before the user confirms「立即更新」.
- No full About page.
- No change to Skill Hub startup refresh.

## Error Handling

- Update check failure: ignore (existing); no tag.
- Version fetch failure: hide version text; update tag can still appear if check succeeded.
- Install failure: show error inside dialog (existing); dialog stays open; tag remains.

## Testing

1. Renders `v{version}` in sidebar footer when version is available.
2. Startup with update available → tag visible, dialog closed.
3. Click tag → dialog opens with correct version notes.
4. 「稍后」→ dialog closes, tag still visible; click tag opens dialog again.
5. No update → no tag.
6. Existing install / error paths still covered.

## Acceptance

- User can read the installed version without leaving the main UI.
- New releases never interrupt with an automatic modal; only the sidebar tag appears.
- Existing update dialog remains the confirmation surface for install.
