import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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
    expect(screen.getByTestId('gitlab-host-gitlab.internal')).toHaveTextContent(
      'platform/tools',
    );
    expect(screen.queryByText('acme/public-skills')).not.toBeInTheDocument();
  });

  it('shows an empty state when no GitLab repositories are configured', () => {
    renderDialog([githubRepo]);
    expect(screen.getByText('暂无已添加的 GitLab 来源仓库')).toBeInTheDocument();
  });

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
    expect(within(authenticated).getByRole('button', { name: '删除' })).toBeInTheDocument();

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
    await waitFor(() =>
      expect(
        screen.queryByRole('dialog', { name: '删除 GitLab 访问密钥' }),
      ).not.toBeInTheDocument(),
    );
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
});
