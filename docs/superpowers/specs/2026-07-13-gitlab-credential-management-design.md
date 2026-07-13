# GitLab Credential Management Design

## Goal

Make GitLab authentication recoverable and understandable after a PAT is removed. The credential management dialog must show every configured GitLab repository, group repositories by GitLab host, and always provide an authentication entry point for hosts without a stored PAT.

## Scope

This first phase uses only states the application can determine reliably:

- `已认证`: the host has a stored PAT.
- `未配置认证`: the host has no stored PAT.

The UI must not claim that a repository is public, private, accessible, or inaccessible. The current repository model does not persist access requirements or the result of the latest access check.

## User Interface

Replace the current credential-only list in the key management dialog with a GitLab host list derived from all configured GitLab repositories.

Each host is displayed once and contains:

- The normalized GitLab host name.
- Authentication status.
- The number of repositories using the host.
- A list of the repositories' project paths.
- Host-level authentication actions.

Actions depend on status:

- An authenticated host provides `更新` and `删除`.
- A host without a configured PAT provides `去认证`.

Repositories remain visible after credential deletion. Only the host status and available actions change.

If there are no configured GitLab repositories, the dialog displays an empty-state message. Credential entries that no longer correspond to a configured GitLab repository are outside this phase and are not displayed.

## Authentication Flow

Selecting `去认证` or `更新` opens the existing GitLab PAT dialog above the key management dialog.

The PAT dialog:

- Identifies the GitLab host being configured.
- Validates the PAT before saving it.
- Remains open while validation and saving are in progress.
- Displays validation or storage failures inside the dialog.
- Closes only after validation and persistence both succeed.

After success, the host group immediately changes to `已认证`. Because credentials are host-scoped, every repository in that group shares the new status.

## Credential Deletion

Selecting `删除` opens a confirmation dialog above the key management dialog. The confirmation includes:

- The GitLab host.
- The number of affected repositories.
- The affected repository project paths.
- A warning that those repositories may become inaccessible.
- A note that authentication can be configured again through `去认证`.

Deletion is disabled while the operation is running. The key management dialog remains open.

On success, the host group changes to `未配置认证` and exposes `去认证`. On failure, the credential remains shown as authenticated and the error is displayed in the active confirmation or key-management layer.

## Data Flow

`SourceManageDrawer` already owns both required inputs:

- The configured repository list.
- The list of GitLab hosts with stored credentials.

It passes the GitLab repositories and configured host list to `KeysManageDialog`. The dialog groups repositories by normalized host and derives each group's authentication status from the configured host set.

No new persistent repository fields or access-status API are introduced in this phase.

After an authentication update or deletion, `SourceManageDrawer` reloads configured credential hosts and passes the updated state back to the dialog.

## Component Responsibilities

### SourceManageDrawer

- Supplies configured GitLab repositories to the key management dialog.
- Owns PAT dialog state and host credential mutations.
- Refreshes configured hosts after successful mutations.
- Coordinates nested dialog visibility and layer ordering.

### KeysManageDialog

- Groups repositories by normalized GitLab host.
- Renders host status, repository membership, and host-level actions.
- Requests authentication, update, or deletion through callbacks.
- Owns the delete-confirmation presentation state, but not credential persistence.

### GitLabPatDialog

- Continues to validate and submit a PAT.
- Displays submission errors locally.
- Uses copy appropriate to authentication or update mode.

## Layering and Keyboard Behavior

Nested layers follow this order:

1. Source management drawer.
2. Key management dialog.
3. PAT or delete confirmation dialog.
4. Global toast notifications.

Escape closes only the topmost active layer. Clicking a parent overlay must not close a child dialog or the source drawer. A completed child operation returns focus to the corresponding host action in the key management dialog where practical.

## Backend Correction

Credential deletion must be atomic from the UI's perspective:

1. Delete the PAT from the operating-system credential store.
2. Only after successful deletion, remove the host from application configuration.
3. Persist the updated application configuration.

Credential-store deletion errors must be returned instead of ignored. If deletion fails, the configured host record remains unchanged so the UI does not report a false `未配置认证` state.

If credential-store deletion succeeds but configuration persistence fails, the operation returns an error and the next credential reconciliation reflects the actual credential-store state. Full rollback into the OS credential store is not attempted because the deleted secret is no longer available.

## Error Handling

- PAT validation and saving errors appear inside the PAT dialog.
- Delete errors appear in the topmost deletion/key-management context.
- Failed operations retain the current dialog and user context.
- Success may use a non-blocking toast, but the status change in the host group is the primary confirmation.

## Testing

Frontend tests cover:

- Grouping multiple repositories under the same normalized host.
- Rendering all configured GitLab repositories and excluding GitHub repositories.
- `已认证` with `更新` and `删除` actions.
- `未配置认证` with a `去认证` action.
- Opening the PAT dialog for authentication and update.
- Updating host status after successful PAT persistence.
- Delete confirmation content for one and multiple repositories.
- Retaining authenticated status after deletion failure.
- Showing an empty state when no GitLab repositories exist.
- Ensuring Escape affects only the topmost dialog.

Backend tests cover:

- Successful credential deletion unregisters the host.
- Credential-store deletion failure is propagated and does not unregister the host.

## Deferred Work

The following are explicitly outside this phase:

- Distinguishing public repositories from private repositories.
- Persisting repository access health.
- Detecting expired or insufficient PAT permissions outside an attempted operation.
- Automatically probing every repository when the key management dialog opens.
- Displaying orphaned credential hosts that have no configured repositories.
