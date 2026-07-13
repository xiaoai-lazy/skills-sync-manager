# GitLab Credential Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the credential-only GitLab key list with host-grouped repository authentication management that always offers a recovery path after PAT deletion.

**Architecture:** Keep repository and credential data ownership in `SourceManageDrawer`, and make `KeysManageDialog` a host-grouped presentation and delete-confirmation component. Derive authentication status from configured credential hosts without adding persistent repository state. Make credential deletion report keyring failures before changing configuration, then wire the existing PAT dialog as the add/update flow above the still-open key dialog.

**Tech Stack:** React 18, TypeScript, Testing Library, Vitest, Tauri 2, Rust, OS keyring crate, CSS.

---

## File Map

- Modify `src-tauri/src/credential_store.rs`: make host unregistration fallible and test deletion-order behavior with an injected deletion function.
- Modify `src-tauri/src/commands/skill_hub.rs`: propagate credential deletion errors and save configuration only after keyring deletion succeeds.
- Create `src/test/KeysManageDialog.test.tsx`: cover host grouping, actions, confirmation, local errors, and Escape behavior.
- Modify `src/components/skill-hub/KeysManageDialog.tsx`: render GitLab repositories grouped by normalized host and own delete-confirmation state.
- Modify `src/components/skill-hub/SourceManageDrawer.tsx`: pass repositories into key management, keep key management open during PAT operations, and reload credential state after mutations.
- Modify `src/components/skill-hub/GitLabPatDialog.tsx`: add host-authentication copy and a nested overlay class.
- Modify `src/test/GitLabPatDialog.test.tsx`: cover host-authentication copy and nested overlay class.
- Modify `src/test/SkillHubPage.test.tsx`: verify end-to-end key-management authentication and removal calls through the Tauri API mock.
- Modify `src/styles/overlays.css`: style grouped hosts, repository membership, confirmation content, local errors, and nested layer ordering.

### Task 1: Make Credential Deletion Fail Before Configuration Mutation

**Files:**
- Modify: `src-tauri/src/credential_store.rs:62`
- Modify: `src-tauri/src/commands/skill_hub.rs:464`
- Test: `src-tauri/src/credential_store.rs:179`

- [ ] **Step 1: Replace the existing unregister test with success and failure tests**

Add a private injected helper test target and specify its required behavior through these tests in `credential_store.rs`:

```rust
#[test]
fn unregister_gitlab_host_removes_config_after_token_delete_succeeds() {
    let host = "unregister.example.test";
    let mut config = AppConfig::default();
    register_gitlab_host(&mut config, host);

    let mut deleted = false;
    unregister_gitlab_host_with(&mut config, host, |_| {
        deleted = true;
        Ok(())
    })
    .expect("unregister");

    assert!(deleted);
    assert!(config.gitlab_credential_hosts.is_empty());
}

#[test]
fn unregister_gitlab_host_keeps_config_when_token_delete_fails() {
    let host = "unregister.example.test";
    let mut config = AppConfig::default();
    register_gitlab_host(&mut config, host);

    let error = unregister_gitlab_host_with(&mut config, host, |_| {
        Err(AppError::CredentialStore {
            message: "delete failed".to_string(),
        })
    })
    .expect_err("delete should fail");

    assert!(matches!(error, AppError::CredentialStore { .. }));
    assert_eq!(config.gitlab_credential_hosts, vec![host.to_string()]);
}
```

- [ ] **Step 2: Run the focused Rust tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml credential_store::tests::unregister_gitlab_host
```

Expected: compilation fails because `unregister_gitlab_host_with` does not exist and the current `unregister_gitlab_host` cannot return deletion errors.

- [ ] **Step 3: Implement fallible ordered unregistration**

Replace the current `unregister_gitlab_host` function with:

```rust
fn unregister_gitlab_host_with<F>(
    config: &mut AppConfig,
    host: &str,
    delete_token: F,
) -> Result<(), AppError>
where
    F: FnOnce(&str) -> Result<(), AppError>,
{
    delete_token(host)?;
    let normalized = normalize_host(host);
    config
        .gitlab_credential_hosts
        .retain(|existing| existing != &normalized);
    Ok(())
}

pub fn unregister_gitlab_host(config: &mut AppConfig, host: &str) -> Result<(), AppError> {
    unregister_gitlab_host_with(config, host, remove_gitlab_token)
}
```

Update the Tauri command to propagate the error before saving:

```rust
#[tauri::command]
pub fn remove_gitlab_credential(app: AppHandle, host: String) -> Result<(), AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    credential_store::unregister_gitlab_host(&mut config, &host)
        .map_err(|err| err.to_dto())?;
    store.save(&config).map_err(|err| err.to_dto())
}
```

- [ ] **Step 4: Run the focused and complete Rust tests and verify GREEN**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml credential_store::tests::unregister_gitlab_host
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: both commands pass; failure injection retains the configured host.

- [ ] **Step 5: Commit the backend correction**

```powershell
git add -- src-tauri/src/credential_store.rs src-tauri/src/commands/skill_hub.rs
git commit -m "fix(credentials): propagate GitLab PAT deletion failures"
```

### Task 2: Render GitLab Repositories Grouped by Host

**Files:**
- Create: `src/test/KeysManageDialog.test.tsx`
- Modify: `src/components/skill-hub/KeysManageDialog.tsx`
- Modify: `src/styles/overlays.css:226`

- [ ] **Step 1: Create failing grouping and empty-state tests**

Create `src/test/KeysManageDialog.test.tsx` with reusable repository fixtures and the first two behaviors:

```tsx
import { afterEach, cleanup, describe, expect, it, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import KeysManageDialog from '../components/skill-hub/KeysManageDialog';
import type { SkillRepo } from '../model/types';

const gitlabRepo = (host: string, projectPath: string): SkillRepo => ({
  host,
  provider: 'gitlab',
  projectPath,
  owner: projectPath.split('/')[0],
  name: projectPath.split('/').at(-1) ?? projectPath,
  branch: 'main',
  enabled: true,
});

const githubRepo: SkillRepo = {
  host: 'github.com',
  provider: 'github',
  projectPath: 'acme/public-skills',
  owner: 'acme',
  name: 'public-skills',
  branch: 'main',
  enabled: true,
};

function renderDialog(repos: SkillRepo[], configuredHosts: string[] = []) {
  return render(
    <KeysManageDialog
      open
      repos={repos}
      configuredHosts={configuredHosts}
      nestedDialogOpen={false}
      onClose={vi.fn()}
      onAuthenticate={vi.fn()}
      onUpdate={vi.fn()}
      onRemove={vi.fn().mockResolvedValue(undefined)}
    />,
  );
}

afterEach(cleanup);

describe('KeysManageDialog', () => {
  it('groups GitLab repositories by normalized host and excludes GitHub', () => {
    renderDialog([
      gitlabRepo('GitLab.Example.COM', 'team/skills'),
      gitlabRepo('gitlab.example.com', 'team/docs'),
      gitlabRepo('gitlab.internal', 'platform/tools'),
      githubRepo,
    ]);

    const exampleGroup = screen.getByTestId('gitlab-host-gitlab.example.com');
    expect(within(exampleGroup).getByText('team/skills')).toBeInTheDocument();
    expect(within(exampleGroup).getByText('team/docs')).toBeInTheDocument();
    expect(within(exampleGroup).getByText('2 个仓库')).toBeInTheDocument();
    expect(screen.getByTestId('gitlab-host-gitlab.internal')).toHaveTextContent('platform/tools');
    expect(screen.queryByText('acme/public-skills')).not.toBeInTheDocument();
  });

  it('shows an empty state when no GitLab repositories are configured', () => {
    renderDialog([githubRepo]);
    expect(screen.getByText('暂无已添加的 GitLab 来源仓库')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run the component test and verify RED**

Run:

```powershell
npm test -- --run src/test/KeysManageDialog.test.tsx
```

Expected: TypeScript/render failure because the component does not accept `repos`, `configuredHosts`, `nestedDialogOpen`, or `onAuthenticate`.

- [ ] **Step 3: Implement host grouping and status rendering**

Replace the component props and add the grouping helper:

```tsx
import { useEffect, useMemo, useState } from 'react';
import type { SkillRepo } from '../../model/types';
import { errorMessage } from '../../utils/errorMessage';

export interface KeysManageDialogProps {
  open: boolean;
  repos: SkillRepo[];
  configuredHosts: string[];
  nestedDialogOpen: boolean;
  onClose: () => void;
  onAuthenticate: (host: string) => void;
  onUpdate: (host: string) => void;
  onRemove: (host: string) => Promise<void>;
}

interface GitLabHostGroup {
  host: string;
  repos: SkillRepo[];
  authenticated: boolean;
}

function normalizeHost(host: string): string {
  return host.trim().toLowerCase();
}

function groupGitLabRepos(repos: SkillRepo[], configuredHosts: string[]): GitLabHostGroup[] {
  const authenticatedHosts = new Set(configuredHosts.map(normalizeHost));
  const grouped = new Map<string, SkillRepo[]>();

  for (const repo of repos) {
    if (repo.provider !== 'gitlab') continue;
    const host = normalizeHost(repo.host);
    if (!host) continue;
    grouped.set(host, [...(grouped.get(host) ?? []), repo]);
  }

  return [...grouped.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([host, hostRepos]) => ({
      host,
      repos: [...hostRepos].sort((left, right) =>
        left.projectPath.localeCompare(right.projectPath),
      ),
      authenticated: authenticatedHosts.has(host),
    }));
}
```

Inside the component, derive `groups` with `useMemo` and render each group with this structure:

```tsx
const groups = useMemo(
  () => groupGitLabRepos(repos, configuredHosts),
  [repos, configuredHosts],
);

{groups.length === 0 ? (
  <p className="keys-empty">暂无已添加的 GitLab 来源仓库</p>
) : (
  <div className="credential-host-list">
    {groups.map((group) => (
      <section
        key={group.host}
        className="credential-host"
        data-testid={`gitlab-host-${group.host}`}
      >
        <div className="credential-host-header">
          <div>
            <strong>{group.host}</strong>
            <div className={`credential-status${group.authenticated ? ' authenticated' : ''}`}>
              {group.authenticated ? '已认证' : '未配置认证'} · {group.repos.length} 个仓库
            </div>
          </div>
          <div className="key-actions">
            {group.authenticated ? (
              <>
                <button type="button" onClick={() => onUpdate(group.host)}>更新</button>
                <button type="button" onClick={() => setDeleteGroup(group)}>删除</button>
              </>
            ) : (
              <button type="button" className="btn-primary" onClick={() => onAuthenticate(group.host)}>
                去认证
              </button>
            )}
          </div>
        </div>
        <ul className="credential-repo-list">
          {group.repos.map((repo) => <li key={repo.projectPath}>{repo.projectPath}</li>)}
        </ul>
      </section>
    ))}
  </div>
)}
```

Declare the state required by the render now; deletion behavior is implemented in Task 3:

```tsx
const [deleteGroup, setDeleteGroup] = useState<GitLabHostGroup | null>(null);
const [removing, setRemoving] = useState(false);
const [removeError, setRemoveError] = useState<string | null>(null);
```

- [ ] **Step 4: Add the grouped host styles**

Replace the old `.key-item`, `.key-item-left`, and `.key-status` rules in `overlays.css` with:

```css
.credential-host-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.credential-host {
  padding: 12px;
  border: 1px solid var(--border-soft);
  border-radius: var(--radius-md);
  background: var(--surface-muted);
}

.credential-host-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.credential-host-header strong {
  display: block;
  font-size: 13px;
  color: var(--text);
}

.credential-status {
  margin-top: 3px;
  font-size: 11px;
  color: var(--text-tertiary);
}

.credential-status.authenticated {
  color: var(--success);
  font-weight: 600;
}

.credential-repo-list {
  margin: 10px 0 0;
  padding: 8px 0 0;
  border-top: 1px solid var(--border-soft);
  list-style: none;
}

.credential-repo-list li {
  overflow: hidden;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.7;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 5: Run the component test and verify GREEN**

Run:

```powershell
npm test -- --run src/test/KeysManageDialog.test.tsx
```

Expected: both grouping tests pass.

- [ ] **Step 6: Commit host grouping**

```powershell
git add -- src/components/skill-hub/KeysManageDialog.tsx src/styles/overlays.css src/test/KeysManageDialog.test.tsx
git commit -m "feat(credentials): group GitLab repositories by host"
```

### Task 3: Add Authentication Actions and Safe Delete Confirmation

**Files:**
- Modify: `src/test/KeysManageDialog.test.tsx`
- Modify: `src/components/skill-hub/KeysManageDialog.tsx`
- Modify: `src/styles/overlays.css`

- [ ] **Step 1: Add failing action and confirmation tests**

Add these tests to `KeysManageDialog.test.tsx`:

```tsx
import userEvent from '@testing-library/user-event';
import { waitFor } from '@testing-library/react';

it('offers update and delete for authenticated hosts and authentication for unconfigured hosts', async () => {
  const onAuthenticate = vi.fn();
  const onUpdate = vi.fn();
  const user = userEvent.setup();
  render(
    <KeysManageDialog
      open
      repos={[
        gitlabRepo('gitlab.example.com', 'team/skills'),
        gitlabRepo('gitlab.internal', 'platform/tools'),
      ]}
      configuredHosts={['GITLAB.EXAMPLE.COM']}
      nestedDialogOpen={false}
      onClose={vi.fn()}
      onAuthenticate={onAuthenticate}
      onUpdate={onUpdate}
      onRemove={vi.fn().mockResolvedValue(undefined)}
    />,
  );

  const authenticated = screen.getByTestId('gitlab-host-gitlab.example.com');
  expect(within(authenticated).getByText(/已认证/)).toBeInTheDocument();
  await user.click(within(authenticated).getByRole('button', { name: '更新' }));
  expect(onUpdate).toHaveBeenCalledWith('gitlab.example.com');

  const unconfigured = screen.getByTestId('gitlab-host-gitlab.internal');
  expect(within(unconfigured).getByText(/未配置认证/)).toBeInTheDocument();
  await user.click(within(unconfigured).getByRole('button', { name: '去认证' }));
  expect(onAuthenticate).toHaveBeenCalledWith('gitlab.internal');
});

it('confirms deletion with every affected repository before removing the host', async () => {
  const onRemove = vi.fn().mockResolvedValue(undefined);
  const user = userEvent.setup();
  render(
    <KeysManageDialog
      open
      repos={[
        gitlabRepo('gitlab.example.com', 'team/skills'),
        gitlabRepo('gitlab.example.com', 'team/docs'),
      ]}
      configuredHosts={['gitlab.example.com']}
      nestedDialogOpen={false}
      onClose={vi.fn()}
      onAuthenticate={vi.fn()}
      onUpdate={vi.fn()}
      onRemove={onRemove}
    />,
  );

  await user.click(screen.getByRole('button', { name: '删除' }));
  const confirmation = screen.getByRole('dialog', { name: '删除 GitLab 访问密钥' });
  expect(confirmation).toHaveTextContent('gitlab.example.com');
  expect(confirmation).toHaveTextContent('2 个仓库');
  expect(confirmation).toHaveTextContent('team/skills');
  expect(confirmation).toHaveTextContent('team/docs');
  expect(onRemove).not.toHaveBeenCalled();

  await user.click(within(confirmation).getByRole('button', { name: '确认删除' }));
  await waitFor(() => expect(onRemove).toHaveBeenCalledWith('gitlab.example.com'));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: '删除 GitLab 访问密钥' })).not.toBeInTheDocument());
});

it('keeps the confirmation open and shows a local error when deletion fails', async () => {
  const user = userEvent.setup();
  render(
    <KeysManageDialog
      open
      repos={[gitlabRepo('gitlab.example.com', 'team/skills')]}
      configuredHosts={['gitlab.example.com']}
      nestedDialogOpen={false}
      onClose={vi.fn()}
      onAuthenticate={vi.fn()}
      onUpdate={vi.fn()}
      onRemove={vi.fn().mockRejectedValue(new Error('无法删除 GitLab 凭证'))}
    />,
  );

  await user.click(screen.getByRole('button', { name: '删除' }));
  await user.click(screen.getByRole('button', { name: '确认删除' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('无法删除 GitLab 凭证');
  expect(screen.getByRole('dialog', { name: '删除 GitLab 访问密钥' })).toBeInTheDocument();
  expect(screen.getByTestId('gitlab-host-gitlab.example.com')).toHaveTextContent('已认证');
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
npm test -- --run src/test/KeysManageDialog.test.tsx
```

Expected: action rendering may pass after Task 2, but confirmation tests fail because no nested confirmation dialog or local error handling exists.

- [ ] **Step 3: Implement deletion submission and topmost Escape behavior**

Add this handler inside `KeysManageDialog`:

```tsx
const handleRemove = async () => {
  if (!deleteGroup || removing) return;
  setRemoveError(null);
  setRemoving(true);
  try {
    await onRemove(deleteGroup.host);
    setDeleteGroup(null);
  } catch (err) {
    setRemoveError(errorMessage(err));
  } finally {
    setRemoving(false);
  }
};
```

Change the dialog Escape listener so only the topmost layer closes:

```tsx
useEffect(() => {
  if (!open) return;
  const handleKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return;
    if (deleteGroup) {
      if (!removing) setDeleteGroup(null);
      return;
    }
    if (!nestedDialogOpen) onClose();
  };
  document.addEventListener('keydown', handleKeyDown);
  return () => document.removeEventListener('keydown', handleKeyDown);
}, [open, onClose, deleteGroup, removing, nestedDialogOpen]);
```

Render this confirmation after the main modal overlay:

```tsx
{deleteGroup && (
  <div
    className="modal-overlay open credential-confirm-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="credentialDeleteTitle"
    onClick={() => {
      if (!removing) setDeleteGroup(null);
    }}
  >
    <div className="modal" onClick={(event) => event.stopPropagation()}>
      <h3 id="credentialDeleteTitle">删除 GitLab 访问密钥</h3>
      <p>
        删除 <strong>{deleteGroup.host}</strong> 的访问密钥后，以下 {deleteGroup.repos.length} 个仓库可能无法访问：
      </p>
      <ul className="credential-delete-repos">
        {deleteGroup.repos.map((repo) => <li key={repo.projectPath}>{repo.projectPath}</li>)}
      </ul>
      <p className="credential-delete-note">删除后可通过“去认证”重新配置。</p>
      {removeError && <p className="modal-error show" role="alert">{removeError}</p>}
      <div className="modal-actions">
        <button type="button" className="cancel" disabled={removing} onClick={() => setDeleteGroup(null)}>
          取消
        </button>
        <button type="button" className="danger-button" disabled={removing} onClick={() => void handleRemove()}>
          {removing ? '删除中…' : '确认删除'}
        </button>
      </div>
    </div>
  </div>
)}
```

- [ ] **Step 4: Style the confirmation and nested layer**

Add to `overlays.css`:

```css
.credential-confirm-overlay,
.credential-pat-overlay {
  z-index: 130;
}

.credential-delete-repos {
  max-height: 180px;
  margin: 0 0 12px;
  padding: 10px 12px 10px 28px;
  overflow-y: auto;
  border-radius: var(--radius-md);
  background: var(--surface-muted);
  color: var(--text-secondary);
  font-size: 13px;
}

.credential-delete-note {
  font-size: 12px !important;
  color: var(--text-tertiary) !important;
}
```

- [ ] **Step 5: Run the focused test and verify GREEN**

Run:

```powershell
npm test -- --run src/test/KeysManageDialog.test.tsx
```

Expected: grouping, action, confirmation, and failure tests all pass.

- [ ] **Step 6: Commit delete confirmation behavior**

```powershell
git add -- src/components/skill-hub/KeysManageDialog.tsx src/styles/overlays.css src/test/KeysManageDialog.test.tsx
git commit -m "feat(credentials): confirm GitLab PAT removal"
```

### Task 4: Wire Add/Update Authentication Through the PAT Dialog

**Files:**
- Modify: `src/components/skill-hub/SourceManageDrawer.tsx:47,245,316,325,637`
- Modify: `src/components/skill-hub/GitLabPatDialog.tsx:4,64,76`
- Modify: `src/test/GitLabPatDialog.test.tsx`
- Modify: `src/test/SkillHubPage.test.tsx`

- [ ] **Step 1: Add failing PAT copy and layer tests**

Add to `GitLabPatDialog.test.tsx`:

```tsx
it('describes host authentication without claiming the repository is private', () => {
  render(
    <GitLabPatDialog
      open
      host="gitlab.example.com"
      description="gitlab.example.com"
      mode="authenticate"
      onClose={vi.fn()}
      onSubmit={vi.fn()}
      submitLabel="验证并保存"
    />,
  );

  expect(screen.getByText(/为 GitLab 站点/)).toHaveTextContent('gitlab.example.com');
  expect(screen.queryByText(/需要登录后访问/)).not.toBeInTheDocument();
  expect(screen.getByRole('dialog')).toHaveClass('credential-pat-overlay');
});
```

- [ ] **Step 2: Add failing integration tests for authenticate, update, and delete refresh**

Extend the mock setup in `SkillHubPage.test.tsx` so `update_gitlab_credential` and `remove_gitlab_credential` resolve successfully. Add tests that open 来源管理 then 密钥管理:

```tsx
it('authenticates an unconfigured GitLab host from key management', async () => {
  const user = userEvent.setup();
  setupInvokeMocks([mockGitLabRepo]);
  renderHub();

  await user.click(screen.getByRole('button', { name: '来源管理' }));
  await user.click(screen.getByRole('button', { name: '密钥管理' }));
  await user.click(await screen.findByRole('button', { name: '去认证' }));

  expect(screen.getByRole('dialog', { name: '配置 GitLab 访问密钥' })).toBeInTheDocument();
  await user.type(screen.getByLabelText('访问密钥（PAT）'), 'glpat-test');
  await user.click(screen.getByRole('button', { name: '验证并保存' }));

  await waitFor(() => {
    expect(invokeMock).toHaveBeenCalledWith('update_gitlab_credential', {
      host: 'gitlab.example.com',
      pat: 'glpat-test',
    });
  });
  expect(screen.getByRole('dialog', { name: '密钥管理' })).toBeInTheDocument();
});

it('refreshes the host to unconfigured after confirmed credential deletion', async () => {
  const user = userEvent.setup();
  let credentialHosts = ['gitlab.example.com'];
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'get_skill_repos') return Promise.resolve([mockGitLabRepo]);
    if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
    if (cmd === 'list_gitlab_credentials') return Promise.resolve(credentialHosts);
    if (cmd === 'remove_gitlab_credential') {
      credentialHosts = [];
      return Promise.resolve();
    }
    return Promise.resolve(null);
  });
  renderHub();

  await user.click(screen.getByRole('button', { name: '来源管理' }));
  await user.click(screen.getByRole('button', { name: '密钥管理' }));
  await user.click(await screen.findByRole('button', { name: '删除' }));
  await user.click(screen.getByRole('button', { name: '确认删除' }));

  expect(await screen.findByRole('button', { name: '去认证' })).toBeInTheDocument();
  expect(screen.getByText(/未配置认证/)).toBeInTheDocument();
});
```

- [ ] **Step 3: Run the focused frontend tests and verify RED**

Run:

```powershell
npm test -- --run src/test/GitLabPatDialog.test.tsx src/test/SkillHubPage.test.tsx
```

Expected: `authenticate` is not a valid PAT mode, the key dialog lacks repository props, and the current update handler closes key management.

- [ ] **Step 4: Extend PAT mode and host-authentication copy**

Change the PAT dialog mode type:

```tsx
mode?: 'add' | 'authenticate' | 'update';
```

Use this description selection:

```tsx
const descText =
  mode === 'update' ? (
    <>
      更新 <strong>{description}</strong> 的访问密钥。请输入新的个人访问令牌（PAT）。
    </>
  ) : mode === 'authenticate' ? (
    <>
      为 GitLab 站点 <strong>{description}</strong> 配置个人访问令牌（PAT）。同一站点下的来源仓库共用此密钥。
    </>
  ) : (
    <>
      仓库 <strong>{description}</strong> 需要登录后访问。请输入对该站点有读权限的个人访问令牌（PAT）。
    </>
  );
```

Add the class to its overlay:

```tsx
className="modal-overlay open credential-pat-overlay"
```

- [ ] **Step 5: Keep key management open and wire repository data**

Extend `PatDialogState`:

```tsx
mode: 'add' | 'authenticate' | 'update';
```

Replace credential handlers with:

```tsx
const handleRemoveCredential = async (host: string) => {
  await removeGitlabCredential(host);
  await loadConfiguredHosts();
};

const handleAuthenticateCredential = (host: string) => {
  setPatDialog({ host, url: '', projectPath: host, mode: 'authenticate' });
};

const handleUpdateCredential = (host: string) => {
  setPatDialog({ host, url: '', projectPath: host, mode: 'update' });
};
```

Change PAT submission so both credential-management modes await save and refresh:

```tsx
if (mode === 'authenticate' || mode === 'update') {
  await updateGitlabCredential(host, pat);
  await loadConfiguredHosts();
  return;
}
```

Pass the new key-management props:

```tsx
<KeysManageDialog
  open={keysDialogOpen}
  repos={repos}
  configuredHosts={configuredHosts}
  nestedDialogOpen={patDialog !== null}
  onClose={() => setKeysDialogOpen(false)}
  onAuthenticate={handleAuthenticateCredential}
  onUpdate={handleUpdateCredential}
  onRemove={handleRemoveCredential}
/>
```

Use host-management copy and labels for the PAT dialog:

```tsx
mode={patDialog?.mode ?? 'add'}
submitLabel={patDialog?.mode === 'add' ? '验证并添加' : '验证并保存'}
```

- [ ] **Step 6: Run focused frontend tests and verify GREEN**

Run:

```powershell
npm test -- --run src/test/KeysManageDialog.test.tsx src/test/GitLabPatDialog.test.tsx src/test/SkillHubPage.test.tsx
```

Expected: all credential-management component and integration tests pass.

- [ ] **Step 7: Commit authentication recovery flow**

```powershell
git add -- src/components/skill-hub/SourceManageDrawer.tsx src/components/skill-hub/GitLabPatDialog.tsx src/test/GitLabPatDialog.test.tsx src/test/SkillHubPage.test.tsx
git commit -m "feat(credentials): restore GitLab authentication from key management"
```

### Task 5: Verify Topmost Escape and Complete Regression Coverage

**Files:**
- Modify: `src/test/KeysManageDialog.test.tsx`
- Modify: `src/test/SkillHubPage.test.tsx`

- [ ] **Step 1: Add a failing topmost Escape regression test**

Add to `SkillHubPage.test.tsx`:

```tsx
it('Escape closes only the PAT dialog and leaves key management open', async () => {
  const user = userEvent.setup();
  setupInvokeMocks([mockGitLabRepo]);
  renderHub();

  await user.click(screen.getByRole('button', { name: '来源管理' }));
  await user.click(screen.getByRole('button', { name: '密钥管理' }));
  await user.click(await screen.findByRole('button', { name: '去认证' }));
  await user.keyboard('{Escape}');

  expect(screen.queryByRole('dialog', { name: '配置 GitLab 访问密钥' })).not.toBeInTheDocument();
  expect(screen.getByRole('dialog', { name: '密钥管理' })).toBeInTheDocument();
  expect(screen.getByRole('dialog', { name: '来源管理' })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the regression test and verify RED if listener ordering still closes parents**

Run:

```powershell
npm test -- --run src/test/SkillHubPage.test.tsx -t "Escape closes only the PAT dialog"
```

Expected before the final listener correction: FAIL if key management or the source drawer also closes. If it already passes because Task 4 correctly suspends parent listeners, record the passing result and do not add unnecessary production code.

- [ ] **Step 3: Apply the minimal listener correction only if RED occurs**

If the source drawer closes, retain its existing top-layer guard and ensure it includes all child states:

```tsx
if (event.key === 'Escape' && !patDialog && !keysDialogOpen && !addModalOpen) {
  onClose();
}
```

If key management closes, verify the `nestedDialogOpen={patDialog !== null}` prop and this guard in `KeysManageDialog`:

```tsx
if (!nestedDialogOpen) onClose();
```

Do not introduce another document listener or timeout.

- [ ] **Step 4: Run all frontend tests and production build**

Run:

```powershell
npm test
npm run build
```

Expected: all Vitest files pass; TypeScript and Vite production build complete without errors.

- [ ] **Step 5: Run all Rust tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass, including the injected credential deletion failure test.

- [ ] **Step 6: Inspect the final diff for scope and whitespace errors**

Run:

```powershell
git diff --check
git status --short
git diff --stat
```

Expected: no whitespace errors; only the files listed in this plan are changed by the implementation.

- [ ] **Step 7: Commit final regression coverage if Task 5 changed files**

```powershell
git add -- src/components/skill-hub/SourceManageDrawer.tsx src/components/skill-hub/KeysManageDialog.tsx src/test/KeysManageDialog.test.tsx src/test/SkillHubPage.test.tsx
git commit -m "test(credentials): cover nested GitLab authentication dialogs"
```

If Task 5 required no file changes, skip this commit rather than creating an empty commit.
