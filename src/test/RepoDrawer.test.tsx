import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import RepoDrawer from '../components/skill-hub/RepoDrawer';
import type { SkillRepo } from '../model/types';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const githubRepo: SkillRepo = {
  host: 'github.com',
  provider: 'github',
  projectPath: 'obra/superpowers',
  owner: 'obra',
  name: 'superpowers',
  branch: 'main',
  enabled: true,
};

const gitlabRepo: SkillRepo = {
  host: 'gitlab.example.com',
  provider: 'gitlab',
  projectPath: 'acme/tools',
  owner: 'acme',
  name: 'tools',
  branch: 'main',
  enabled: true,
};

function mockInvokeForOpenDrawer() {
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'get_skill_repos') {
      return Promise.resolve([githubRepo, gitlabRepo]);
    }
    if (cmd === 'list_gitlab_credentials') {
      return Promise.resolve(['gitlab.example.com']);
    }
    return Promise.resolve(null);
  });
}

describe('RepoDrawer', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it('does not render when closed', () => {
    render(<RepoDrawer open={false} onClose={vi.fn()} />);
    expect(screen.queryByRole('dialog', { name: 'Skill 来源仓库' })).not.toBeInTheDocument();
  });

  it('shows drawer title and 密钥管理 button', async () => {
    mockInvokeForOpenDrawer();
    render(<RepoDrawer open={true} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Skill 来源仓库' })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: '密钥管理' })).toBeInTheDocument();
  });

  it('shows 已认证 badge for GitLab repo with configured host', async () => {
    mockInvokeForOpenDrawer();
    render(<RepoDrawer open={true} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('gitlab.example.com/acme/tools')).toBeInTheDocument();
    });
    expect(screen.getByText('已认证')).toBeInTheDocument();
  });

  it('displays GitHub repo with GitHub · path label', async () => {
    mockInvokeForOpenDrawer();
    render(<RepoDrawer open={true} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('obra/superpowers')).toBeInTheDocument();
    });
  });

  it('opens keys manage dialog from header button', async () => {
    mockInvokeForOpenDrawer();
    render(<RepoDrawer open={true} onClose={vi.fn()} />);

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: '密钥管理' })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: '密钥管理' }));

    expect(screen.getByRole('dialog', { name: '密钥管理' })).toBeInTheDocument();
    expect(screen.getByText('GitLab · gitlab.example.com')).toBeInTheDocument();
    expect(screen.getByText('已配置')).toBeInTheDocument();
  });

  it('opens PAT dialog when preview needsPat', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([githubRepo]);
      if (cmd === 'list_gitlab_credentials') return Promise.resolve([]);
      if (cmd === 'preview_add_skill_repo') {
        return Promise.resolve({
          canSave: true,
          needsPat: true,
          host: 'gitlab.example.com',
          provider: 'gitlab',
          projectPath: 'acme/tools',
          branch: 'main',
          error: null,
        });
      }
      if (cmd === 'add_skill_repo') {
        return Promise.resolve({ repos: [githubRepo, gitlabRepo], discoverSkills: [] });
      }
      return Promise.resolve(null);
    });

    render(<RepoDrawer open={true} onClose={vi.fn()} />);

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByLabelText('仓库链接')).toBeInTheDocument();
    });

    await user.type(
      screen.getByLabelText('仓库链接'),
      'https://gitlab.example.com/acme/tools',
    );
    await user.click(screen.getByRole('button', { name: '添加来源' }));

    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: '配置 GitLab 访问密钥' })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: '验证并添加' })).toBeDisabled();
  });

  it('calls removeSkillRepo with host and projectPath', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([gitlabRepo]);
      if (cmd === 'list_gitlab_credentials') return Promise.resolve(['gitlab.example.com']);
      if (cmd === 'remove_skill_repo') {
        return Promise.resolve({ repos: [], discoverSkills: [] });
      }
      return Promise.resolve(null);
    });

    const onDiscoverSkillsChange = vi.fn();
    render(
      <RepoDrawer open={true} onClose={vi.fn()} onDiscoverSkillsChange={onDiscoverSkillsChange} />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /删除 gitlab\.example\.com/ })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /删除 gitlab\.example\.com/ }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('remove_skill_repo', {
        host: 'gitlab.example.com',
        projectPath: 'acme/tools',
      });
    });
  });

  it('calls setSkillRepoEnabled when toggling enable checkbox', async () => {
    const disabledRepo = { ...gitlabRepo, enabled: false };
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([disabledRepo]);
      if (cmd === 'list_gitlab_credentials') return Promise.resolve(['gitlab.example.com']);
      if (cmd === 'set_skill_repo_enabled') {
        return Promise.resolve({
          repos: [{ ...gitlabRepo, enabled: true }],
          discoverSkills: [],
        });
      }
      return Promise.resolve(null);
    });

    const onDiscoverSkillsChange = vi.fn();
    render(
      <RepoDrawer open={true} onClose={vi.fn()} onDiscoverSkillsChange={onDiscoverSkillsChange} />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByText('已停用')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('checkbox', { name: /启用 gitlab\.example\.com/ }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_skill_repo_enabled', {
        host: 'gitlab.example.com',
        projectPath: 'acme/tools',
        enabled: true,
      });
    });
  });
});
