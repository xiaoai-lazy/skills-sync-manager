# Source Management Dialog and Feedback Design

## Goal

Make source management operations predictable and accessible: errors stay with the active dialog, asynchronous work cannot be interrupted or duplicated, successful additions return to the skill list, and nested overlays follow one layering and keyboard model.

## Scope

This change covers the source management drawer, add-source dialog, GitLab PAT dialog, source deletion confirmation, application Toast feedback, and shared dialog focus behavior. It does not redesign the source list or change backend source APIs.

## Interaction Model

- The source drawer is the first modal layer at `z-index: 100`.
- Add-source and key-management dialogs are child layers at `z-index: 120`.
- PAT and source-deletion dialogs are nested layers at `z-index: 130`.
- Global Toast feedback is fixed at `z-index: 1000`.
- Escape closes only the topmost active layer. Escape, overlay clicks, cancellation, and Tab switching are ignored while that layer is submitting.
- Opening a layer moves focus to its intended initial control and traps Tab/Shift+Tab within it. Closing restores focus to the element that opened it.

## Add Source Flow

The add-source dialog owns its validation and API error state. It reserves enough vertical space for one error message so showing an error does not shift the form and action row.

For Skill Hub additions, the dialog awaits `addSkillHubEndpoint`. For GitHub and public GitLab additions, it awaits preview and repository addition. A private GitLab preview opens the PAT dialog without discarding the add-source form.

For private GitLab additions, PAT validation, credential persistence, configured-host refresh, and repository addition form one awaited operation. The PAT dialog remains open and disabled until all steps complete. Any failure is rendered inside the PAT dialog and preserves both the PAT and repository URL. No promise is deliberately detached.

After any source is added successfully, the implementation resets the add form, closes the PAT dialog when present, closes the add-source dialog, closes the source drawer, and emits a success Toast. A failure leaves the active dialog and its input values intact.

## Error and Toast Feedback

Expected field, preview, authentication, addition, and deletion failures appear in the dialog performing the operation with `role="alert"`. Source operations do not route these errors to the main-content error banner.

The application owns a general Toast state separate from migration-report state. Toasts support success and error variants, render outside the main panel, use fixed positioning above every modal, and dismiss automatically or through a close button. Existing Skill Hub `onToast` callbacks connect to this state.

Unexpected source-loading and background settings failures may use the global error Toast because no field-level dialog owns them.

## Source Deletion

Clicking delete for a Skill Hub endpoint or Git repository opens a danger confirmation dialog containing the source name or repository path. The deletion is not started before confirmation.

While deletion is pending, confirmation and cancellation controls are disabled, Escape and overlay dismissal are ignored, duplicate deletion is prevented, and other source actions remain unavailable. On success, the local source list and discoverable skills update and a success Toast appears. On failure, the confirmation dialog stays open and displays the error inline.

Undo is outside this change because the backend does not expose a reliable restore transaction.

## Shared Focus Behavior

A small shared modal-focus hook manages:

- capturing the previously focused element;
- selecting an explicit initial focus target or the first enabled focusable element;
- cycling Tab from the last control to the first and Shift+Tab from the first to the last;
- restoring focus when the layer unmounts;
- optionally handling Escape when closing is allowed.

The common `Dialog` component uses this hook. Source-specific overlays use the same hook without requiring an unrelated visual migration to `Dialog`. Nested layers each capture and restore focus independently, so closing PAT returns to the add-source or key-management control beneath it.

## State Boundaries

`SourceManageDrawer` owns add-form state, add error, active deletion target, deletion error, and operation-busy state. `GitLabPatDialog` continues to own PAT input, submission state, and PAT-flow error. The application owns global Toast presentation.

Only the operation that starts an asynchronous mutation may clear its busy state. Close handlers check that state before resetting forms or unmounting overlays.

## Testing

Component tests will cover:

- add-source Escape behavior and Escape suppression while adding;
- Tab switching and dismissal being disabled during submission;
- inline preview and add errors preserving input;
- private GitLab addition awaiting the repository operation before PAT closes;
- successful addition closing both dialog and drawer and emitting a Toast;
- Hub and repository deletion confirmation, pending disabling, inline failure, and success;
- layer classes for add-source, key-management, PAT, and deletion dialogs;
- initial focus, Tab/Shift+Tab trapping, and focus restoration in the common dialog and source overlays;
- application wiring from Skill Hub Toast callbacks to the fixed global Toast.

Verification includes focused Vitest runs, the complete test suite, and the TypeScript/Vite production build.
