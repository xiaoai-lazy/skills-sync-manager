import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import SourceManageDrawer from '../components/skill-hub/SourceManageDrawer';
import type { IflytekSkillHubEndpoint, SkillHubEndpoint, SkillRepo } from '../model/types';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const gitlabRepo: SkillRepo = {
  host: 'gitlab.example.com',
  projectPath: 'team/skills',
  owner: 'team',
  name: 'skills',
  branch: 'main',
  provider: 'gitlab',
  enabled: true,
};

const hubEndpoint: SkillHubEndpoint = {
  id: 'company-hub',
  name: 'Company Hub',
  baseUrl: 'https://hub.example.com',
  enabled: true,
};

const iflytekEndpoint: IflytekSkillHubEndpoint = {
  id: 'iflytek-1',
  name: '讯飞 Hub',
  baseUrl: 'https://iflytek.example.com',
  enabled: true,
};

function baseCommand(cmd: string) {
  if (cmd === 'get_skill_repos') return Promise.resolve([]);
  if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
  if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
  if (cmd === 'list_gitlab_credentials') return Promise.resolve([]);
  return Promise.resolve(null);
}

function renderDrawer(overrides: Partial<React.ComponentProps<typeof SourceManageDrawer>> = {}) {
  const onClose = vi.fn();
  const onError = vi.fn();
  const onToast = vi.fn();
  const props = {
    open: true,
    onClose,
    onError,
    onToast,
    startupRefreshSettings: {
      github: true,
      gitlab: true,
      skillHub: true,
      iflytekSkillHub: true,
    },
    ...overrides,
  };
  return { ...render(<SourceManageDrawer {...props} />), ...props };
}

async function openRepoAdd(user: ReturnType<typeof userEvent.setup>, tab: 'GitHub' | 'GitLab') {
  await user.click(screen.getByRole('button', { name: '添加来源' }));
  const dialog = screen.getByRole('dialog', { name: '添加来源' });
  await user.click(within(dialog).getByRole('tab', { name: tab }));
  return dialog;
}

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation(baseCommand);
});

afterEach(() => cleanup());

describe('SourceManageDrawer dual-track UI', () => {
  it('shows dual-track filter tabs for Skills Sync and iFlytek', () => {
    renderDrawer();
    expect(screen.getByRole('heading', { name: '来源管理' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Skills Sync' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'iFlytek' })).toBeInTheDocument();
  });

  it('disables add until required hub fields are filled', async () => {
    const user = userEvent.setup();
    renderDrawer();
    await user.click(screen.getByRole('button', { name: '添加来源' }));
    const dialog = screen.getByRole('dialog', { name: '添加来源' });
    const addButton = within(dialog).getByRole('button', { name: '添加' });

    expect(addButton).toBeDisabled();

    await user.type(within(dialog).getByLabelText('名称'), '公司 Hub');
    expect(addButton).toBeDisabled();

    await user.type(within(dialog).getByLabelText('Base URL'), 'https://hub.example.com');
    expect(addButton).toBeEnabled();
  });

  it('shows iFlytek Skill Hub tab when opening add source modal', async () => {
    const user = userEvent.setup();
    renderDrawer();
    await user.click(screen.getByRole('button', { name: '添加来源' }));
    const dialog = screen.getByRole('dialog', { name: '添加来源' });
    expect(within(dialog).getByRole('tab', { name: 'iFlytek' })).toBeInTheDocument();
    expect(within(dialog).getByRole('tab', { name: 'Skills Sync' })).toBeInTheDocument();
  });

  it('shows iFlytek Skill Hub startup refresh checkbox', () => {
    renderDrawer();
    expect(
      screen.getByRole('checkbox', { name: 'iFlytek Skill Hub 启动自动刷新' }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('checkbox', { name: 'Skills Sync Hub 启动自动刷新' }),
    ).toBeInTheDocument();
  });

  it('adds an iFlytek endpoint via name and base URL', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'add_iflytek_skill_hub_endpoint') {
        return Promise.resolve({
          endpoints: [],
          iflytekSkillHubEndpoints: [iflytekEndpoint],
          discoverSkills: [],
        });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const onIflytekEndpointsChange = vi.fn();
    const { onToast, onClose } = renderDrawer({ onIflytekEndpointsChange });
    await user.click(screen.getByRole('button', { name: '添加来源' }));
    const dialog = screen.getByRole('dialog', { name: '添加来源' });
    await user.click(within(dialog).getByRole('tab', { name: 'iFlytek' }));
    await user.type(within(dialog).getByLabelText('名称'), '讯飞 Hub');
    await user.type(within(dialog).getByLabelText('Base URL'), 'https://iflytek.example.com');
    await user.click(within(dialog).getByRole('button', { name: '添加' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('add_iflytek_skill_hub_endpoint', {
        name: '讯飞 Hub',
        baseUrl: 'https://iflytek.example.com',
      });
    });
    expect(onIflytekEndpointsChange).toHaveBeenCalledWith([iflytekEndpoint]);
    expect(onToast).toHaveBeenCalledWith('来源已添加');
    expect(onClose).toHaveBeenCalled();
  });
});

describe('SourceManageDrawer add flow', () => {
  it('shows preview errors inside the add dialog and preserves the repository URL', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'preview_add_skill_repo') {
        return Promise.resolve({
          canSave: false,
          needsPat: false,
          host: null,
          provider: null,
          projectPath: null,
          branch: null,
          error: { code: 'invalid_url', message: '仓库链接无效' },
        });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const { onError } = renderDrawer();
    const dialog = await openRepoAdd(user, 'GitHub');
    const input = within(dialog).getByLabelText('仓库链接');
    await user.type(input, 'https://example.com/not-a-repo');
    await user.click(within(dialog).getByRole('button', { name: '添加' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('仓库链接无效');
    expect(input).toHaveValue('https://example.com/not-a-repo');
    expect(onError).not.toHaveBeenCalled();
  });

  it('closes only the add dialog when Escape is pressed while idle', async () => {
    const user = userEvent.setup();
    const { onClose } = renderDrawer();
    await user.click(screen.getByRole('button', { name: '添加来源' }));

    await user.keyboard('{Escape}');

    expect(screen.queryByRole('dialog', { name: '添加来源' })).not.toBeInTheDocument();
    expect(screen.getByRole('dialog', { name: '来源管理' })).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('traps Tab within the add dialog', async () => {
    const user = userEvent.setup();
    renderDrawer();
    await user.click(screen.getByRole('button', { name: '添加来源' }));
    const dialog = screen.getByRole('dialog', { name: '添加来源' });
    const first = within(dialog).getByRole('tab', { name: 'Skills Sync' });
    // 「添加」在必填未齐前禁用，最后可聚焦控件为「取消」
    const last = within(dialog).getByRole('button', { name: '取消' });

    last.focus();
    await user.tab();
    expect(first).toHaveFocus();

    await user.tab({ shift: true });
    expect(last).toHaveFocus();
  });

  it('disables tab switching and dismissal while repository preview is pending', async () => {
    let resolvePreview!: (value: unknown) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'preview_add_skill_repo') {
        return new Promise((resolve) => {
          resolvePreview = resolve;
        });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    renderDrawer();
    const dialog = await openRepoAdd(user, 'GitHub');
    await user.type(within(dialog).getByLabelText('仓库链接'), 'https://github.com/acme/skills');
    await user.click(within(dialog).getByRole('button', { name: '添加' }));

    expect(within(dialog).getByRole('tab', { name: 'Skills Sync' })).toBeDisabled();
    expect(within(dialog).getByRole('tab', { name: 'iFlytek' })).toBeDisabled();
    expect(within(dialog).getByRole('tab', { name: 'GitLab' })).toBeDisabled();
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog', { name: '添加来源' })).toBeInTheDocument();

    resolvePreview({
      canSave: false,
      needsPat: false,
      host: null,
      provider: null,
      projectPath: null,
      branch: null,
      error: { code: 'stopped', message: '停止测试' },
    });
    await screen.findByRole('alert');
  });

  it('keeps the PAT dialog open until private repository addition finishes', async () => {
    let resolveAdd!: (value: unknown) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'preview_add_skill_repo') {
        return Promise.resolve({
          canSave: true,
          needsPat: true,
          host: 'gitlab.example.com',
          provider: 'gitlab',
          projectPath: 'team/skills',
          branch: 'main',
          error: null,
        });
      }
      if (cmd === 'validate_gitlab_pat' || cmd === 'update_gitlab_credential') {
        return Promise.resolve();
      }
      if (cmd === 'add_skill_repo') {
        return new Promise((resolve) => {
          resolveAdd = resolve;
        });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const { onClose, onToast } = renderDrawer();
    const addDialog = await openRepoAdd(user, 'GitLab');
    await user.type(
      within(addDialog).getByLabelText('仓库链接'),
      'https://gitlab.example.com/team/skills',
    );
    await user.click(within(addDialog).getByRole('button', { name: '添加' }));
    const patDialog = await screen.findByRole('dialog', { name: '配置 GitLab 访问密钥' });
    await user.type(within(patDialog).getByLabelText('访问密钥（PAT）'), 'glpat-test');
    await user.click(within(patDialog).getByRole('button', { name: '验证并添加' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('add_skill_repo', expect.anything()));
    expect(screen.getByRole('dialog', { name: '配置 GitLab 访问密钥' })).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();

    resolveAdd({ repos: [gitlabRepo], discoverSkills: [] });
    await waitFor(() => expect(onClose).toHaveBeenCalledTimes(1));
    expect(onToast).toHaveBeenCalledWith('来源已添加');
    expect(screen.queryByRole('dialog', { name: '配置 GitLab 访问密钥' })).not.toBeInTheDocument();
    expect(screen.queryByRole('dialog', { name: '添加来源' })).not.toBeInTheDocument();
  });

  it('keeps PAT and add dialogs open with a local error when repository addition fails', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'preview_add_skill_repo') {
        return Promise.resolve({
          canSave: true,
          needsPat: true,
          host: 'gitlab.example.com',
          provider: 'gitlab',
          projectPath: 'team/skills',
          branch: 'main',
          error: null,
        });
      }
      if (cmd === 'validate_gitlab_pat' || cmd === 'update_gitlab_credential') {
        return Promise.resolve();
      }
      if (cmd === 'add_skill_repo') return Promise.reject(new Error('仓库添加失败'));
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const { onClose, onError } = renderDrawer();
    const addDialog = await openRepoAdd(user, 'GitLab');
    await user.type(
      within(addDialog).getByLabelText('仓库链接'),
      'https://gitlab.example.com/team/skills',
    );
    await user.click(within(addDialog).getByRole('button', { name: '添加' }));
    const patDialog = await screen.findByRole('dialog', { name: '配置 GitLab 访问密钥' });
    await user.type(within(patDialog).getByLabelText('访问密钥（PAT）'), 'glpat-test');
    await user.click(within(patDialog).getByRole('button', { name: '验证并添加' }));

    expect(await within(patDialog).findByRole('alert')).toHaveTextContent('仓库添加失败');
    expect(screen.getByRole('dialog', { name: '添加来源' })).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });
});

describe('SourceManageDrawer delete flow', () => {
  it('confirms a Hub source before deleting it', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([hubEndpoint]);
      if (cmd === 'remove_skill_hub_endpoint') {
        return Promise.resolve({ endpoints: [], discoverSkills: [] });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const { onToast } = renderDrawer();
    const drawer = screen.getByRole('dialog', { name: '来源管理' });
    await user.click(await within(drawer).findByRole('button', { name: '删除 Company Hub' }));

    expect(invokeMock).not.toHaveBeenCalledWith('remove_skill_hub_endpoint', expect.anything());
    const confirmation = screen.getByRole('dialog', { name: '删除来源' });
    expect(confirmation).toHaveTextContent('Company Hub');
    await user.click(within(confirmation).getByRole('button', { name: '确认删除' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('remove_skill_hub_endpoint', { id: 'company-hub' });
    });
    expect(onToast).toHaveBeenCalledWith('来源已删除');
  });

  it('disables dismissal and duplicate submission while repository deletion is pending', async () => {
    let resolveDelete!: (value: unknown) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([gitlabRepo]);
      if (cmd === 'remove_skill_repo') {
        return new Promise((resolve) => {
          resolveDelete = resolve;
        });
      }
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    renderDrawer();
    const drawer = screen.getByRole('dialog', { name: '来源管理' });
    await user.click(
      await within(drawer).findByRole('button', {
        name: '删除 gitlab.example.com/team/skills',
      }),
    );
    const confirmation = screen.getByRole('dialog', { name: '删除来源' });
    await user.click(within(confirmation).getByRole('button', { name: '确认删除' }));

    expect(within(confirmation).getByRole('button', { name: '删除中…' })).toBeDisabled();
    expect(within(confirmation).getByRole('button', { name: '取消' })).toBeDisabled();
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog', { name: '删除来源' })).toBeInTheDocument();
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === 'remove_skill_repo')).toHaveLength(1);

    resolveDelete({ repos: [], discoverSkills: [] });
    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: '删除来源' })).not.toBeInTheDocument();
    });
  });

  it('keeps the confirmation open and displays repository deletion failures locally', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([gitlabRepo]);
      if (cmd === 'remove_skill_repo') return Promise.reject(new Error('无法删除来源'));
      return baseCommand(cmd);
    });
    const user = userEvent.setup();
    const { onError } = renderDrawer();
    const drawer = screen.getByRole('dialog', { name: '来源管理' });
    await user.click(
      await within(drawer).findByRole('button', {
        name: '删除 gitlab.example.com/team/skills',
      }),
    );
    const confirmation = screen.getByRole('dialog', { name: '删除来源' });
    await user.click(within(confirmation).getByRole('button', { name: '确认删除' }));

    expect(await within(confirmation).findByRole('alert')).toHaveTextContent('无法删除来源');
    expect(screen.getByRole('dialog', { name: '删除来源' })).toBeInTheDocument();
    expect(onError).not.toHaveBeenCalled();
  });
});
