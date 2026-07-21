import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import SkillHubPage from '../components/skill-hub/SkillHubPage';
import { repoNodeId } from '../components/skill-hub/sourceTreeUtils';
import type {
  DiscoverableSkill,
  IflytekSkillHubEndpoint,
  SkillHubEndpoint,
  SkillHubLocalState,
  SkillRepo,
  SkillUpdateInfo,
  SkillView,
} from '../model/types';
import {
  emptyV6DiscoverableFields,
  emptyV6SkillRecordFields,
  emptyV6SkillViewFields,
} from '../model/types';

const invokeMock = vi.fn();
const uploadSkillToHubMock = vi.fn();
const updateSkillMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock('../api/skillHub', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api/skillHub')>();
  return {
    ...actual,
    uploadSkillToHub: (...args: unknown[]) => uploadSkillToHubMock(...args),
    updateSkill: (...args: unknown[]) => updateSkillMock(...args),
  };
});

const mockGitHubRepo: SkillRepo = {
  host: 'github.com',
  projectPath: 'anthropics/skills',
  owner: 'anthropics',
  name: 'skills',
  branch: 'main',
  provider: 'github',
  enabled: true,
};

const mockGitLabRepo: SkillRepo = {
  host: 'gitlab.example.com',
  projectPath: 'team/skills',
  owner: 'team',
  name: 'skills',
  branch: 'main',
  provider: 'gitlab',
  enabled: true,
};

const mockSkills: SkillView[] = [
  {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas before implementation.',
    path: 'C:\\skills\\brainstorming',
    valid: true,
    validationErrors: [],
    ...emptyV6SkillViewFields,
    storageKey: 'local/brainstorming',
    linkName: 'brainstorming',
  },
  {
    dirName: 'broken-skill',
    name: null,
    description: null,
    path: 'C:\\skills\\broken-skill',
    valid: false,
    validationErrors: ['缺少 SKILL.md frontmatter'],
    ...emptyV6SkillViewFields,
    storageKey: 'local/broken-skill',
    linkName: 'broken-skill',
  },
  {
    dirName: 'tdd',
    name: 'Test-Driven Development',
    description: '红-绿-重构循环。',
    path: 'C:\\skills\\tdd',
    valid: true,
    validationErrors: [],
    ...emptyV6SkillViewFields,
    storageKey: 'local/tdd',
    linkName: 'tdd',
  },
  {
    dirName: 'code-review',
    name: 'Code Review',
    description: 'GitLab 来源的 Skill。',
    path: 'C:\\skills\\code-review',
    valid: true,
    validationErrors: [],
    ...emptyV6SkillViewFields,
    storageKey: 'local/code-review',
    linkName: 'code-review',
  },
];

const mockHubState: SkillHubLocalState = {
  skills: mockSkills,
  validCount: 3,
  invalidCount: 1,
  pendingUpdateCount: 1,
  lastScanAt: '2026-06-30T00:00:00Z',
  skillRecords: {
    tdd: {
      repoHost: 'github.com',
      projectPath: 'anthropics/skills',
      source: 'github',
      repoOwner: 'anthropics',
      repoName: 'skills',
      repoBranch: 'main',
      directory: 'skills/tdd',
      contentHash: 'abc',
      installedAt: '2026-06-30T00:00:00Z',
      ...emptyV6SkillRecordFields,
    },
    'code-review': {
      repoHost: 'gitlab.example.com',
      projectPath: 'team/skills',
      source: 'gitlab',
      repoOwner: 'team',
      repoName: 'skills',
      repoBranch: 'main',
      directory: 'skills/code-review',
      contentHash: 'xyz',
      installedAt: '2026-06-30T00:00:00Z',
      ...emptyV6SkillRecordFields,
    },
  },
};

const mockPendingUpdates: SkillUpdateInfo[] = [
  {
    dirName: 'tdd',
    name: 'Test-Driven Development',
    currentHash: 'abc',
    remoteHash: 'def',
    storageKey: 'local/tdd',
  },
];

const mockDiscoverable: DiscoverableSkill = {
  key: 'anthropics/skills:skills/pdf-toolkit',
  name: 'PDF Toolkit',
  description: 'PDF 解析与合并。',
  directory: 'skills/pdf-toolkit',
  installDirName: 'pdf-toolkit',
  repoHost: 'github.com',
  projectPath: 'anthropics/skills',
  repoOwner: 'anthropics',
  repoName: 'skills',
  repoBranch: 'main',
  source: 'github',
  ...emptyV6DiscoverableFields,
};

const mockGitlabDiscoverable: DiscoverableSkill = {
  key: 'gitlab.example.com/team/skills:skills/lint-helper',
  name: 'Lint Helper',
  description: 'GitLab 仓库中的 Skill。',
  directory: 'skills/lint-helper',
  installDirName: 'lint-helper',
  repoHost: 'gitlab.example.com',
  projectPath: 'team/skills',
  repoOwner: 'team',
  repoName: 'skills',
  repoBranch: 'main',
  source: 'gitlab',
  ...emptyV6DiscoverableFields,
};

const hubDirtyStorageKey = 'hub/company-hub/tools/dirty-skill';
const hubDirtySkill: SkillView = {
  dirName: 'dirty-skill',
  name: 'Dirty Skill',
  description: '本地已修改的 Hub Skill。',
  path: 'C:\\skills\\hub\\company-hub\\tools\\dirty-skill',
  valid: true,
  validationErrors: [],
  ...emptyV6SkillViewFields,
  storageKey: hubDirtyStorageKey,
  linkName: 'dirty-skill',
  localDirty: true,
};

const hubCleanStorageKey = 'hub/company-hub/tools/clean-skill';
const hubCleanSkill: SkillView = {
  dirName: 'clean-skill',
  name: 'Clean Skill',
  description: '未修改的 Hub Skill。',
  path: 'C:\\skills\\hub\\company-hub\\tools\\clean-skill',
  valid: true,
  validationErrors: [],
  ...emptyV6SkillViewFields,
  storageKey: hubCleanStorageKey,
  linkName: 'clean-skill',
  localDirty: false,
};

const hubDirtyRecord = {
  repoHost: '',
  projectPath: '',
  source: 'skillhub',
  repoOwner: '',
  repoName: '',
  repoBranch: '',
  directory: 'tools/dirty-skill',
  contentHash: 'local-hash',
  installedAt: '2026-06-30T00:00:00Z',
  ...emptyV6SkillRecordFields,
  storageKey: hubDirtyStorageKey,
  linkName: 'dirty-skill',
  hubEndpointId: 'company-hub',
  hubSkillGroup: 'tools',
  hubSkillId: 'dirty-skill',
};

const hubCleanRecord = {
  ...hubDirtyRecord,
  directory: 'tools/clean-skill',
  storageKey: hubCleanStorageKey,
  linkName: 'clean-skill',
  hubSkillId: 'clean-skill',
};

function hubStateWithSkills(
  skills: SkillView[],
  records: Record<string, (typeof hubDirtyRecord)>,
): SkillHubLocalState {
  const validCount = skills.filter((s) => s.valid).length;
  return {
    skills,
    validCount,
    invalidCount: skills.length - validCount,
    pendingUpdateCount: 0,
    lastScanAt: '2026-06-30T00:00:00Z',
    skillRecords: records,
  };
}

function setupInvokeMocks(repos: SkillRepo[] = [mockGitHubRepo, mockGitLabRepo]) {
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'get_skill_repos') return Promise.resolve(repos);
    if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
    if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
    if (cmd === 'list_gitlab_credentials') return Promise.resolve([]);
    if (cmd === 'list_hub_groups') return Promise.resolve([]);
    if (cmd === 'set_startup_refresh_settings') {
      return Promise.resolve({
        github: true,
        gitlab: true,
        skillHub: true,
        iflytekSkillHub: true,
      });
    }
    if (cmd === 'scan_main_library') {
      return Promise.resolve({
        ...mockHubState,
        lastScanAt: '2026-06-30T01:00:00Z',
      });
    }
    return Promise.resolve(null);
  });
}

function renderHub(overrides: Partial<ComponentProps<typeof SkillHubPage>> = {}) {
  const onHubSkillsRefresh = vi.fn();
  const onDiscoverSkillsChange = vi.fn();
  const onPendingUpdatesChange = vi.fn();
  const onDeleteMainSkill = vi.fn();
  const onSetMainSkillsDir = vi.fn();

  const props = {
    mainSkillsDir: 'C:\\Users\\dev\\.cursor\\skills',
    hubState: mockHubState,
    discoverSkills: [mockDiscoverable],
    pendingUpdates: mockPendingUpdates,
    startupRefreshSettings: {
      github: false,
      gitlab: true,
      skillHub: true,
      iflytekSkillHub: true,
    },
    onHubSkillsRefresh,
    onDiscoverSkillsChange,
    onPendingUpdatesChange,
    onDeleteMainSkill,
    onSetMainSkillsDir,
    onRefreshHub: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };

  const view = render(<SkillHubPage {...props} />);
  return { ...view, ...props };
}

beforeEach(() => {
  invokeMock.mockReset();
  uploadSkillToHubMock.mockReset();
  updateSkillMock.mockReset();
  uploadSkillToHubMock.mockResolvedValue({
    endpoints: [],
    discoverSkills: [],
  });
  updateSkillMock.mockResolvedValue(undefined);
  setupInvokeMocks();
});

afterEach(() => {
  cleanup();
});

describe('SkillHubPage', () => {
  it('shows startup refresh defaults and persists a changed switch', async () => {
    const user = userEvent.setup();
    renderHub();

    await user.click(screen.getByRole('button', { name: '来源管理' }));

    const github = screen.getByRole('checkbox', { name: 'GitHub 启动自动刷新' });
    expect(github).not.toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'GitLab 启动自动刷新' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Skills Sync Hub 启动自动刷新' })).toBeChecked();
    expect(
      screen.getByRole('checkbox', { name: 'iFlytek Skill Hub 启动自动刷新' }),
    ).toBeChecked();

    await user.click(github);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_startup_refresh_settings', {
        settings: {
          github: true,
          gitlab: true,
          skillHub: true,
          iflytekSkillHub: true,
        },
      });
    });
  });

  it('restores the startup refresh switch when saving fails', async () => {
    const user = userEvent.setup();
    const onError = vi.fn();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([]);
      if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
      if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
      if (cmd === 'list_gitlab_credentials') return Promise.resolve([]);
      if (cmd === 'set_startup_refresh_settings') {
        return Promise.reject(new Error('save failed'));
      }
      return Promise.resolve(null);
    });
    renderHub({ onError });

    await user.click(screen.getByRole('button', { name: '来源管理' }));
    const github = screen.getByRole('checkbox', { name: 'GitHub 启动自动刷新' });
    await user.click(github);

    await waitFor(() => expect(onError).toHaveBeenCalled());
    expect(github).not.toBeChecked();
  });

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
      if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
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
    const keysDialog = screen.getByRole('dialog', { name: '密钥管理' });
    await user.click(await within(keysDialog).findByRole('button', { name: '删除' }));
    await user.click(screen.getByRole('button', { name: '确认删除' }));

    expect(await screen.findByRole('button', { name: '去认证' })).toBeInTheDocument();
    expect(screen.getByText(/未配置认证/)).toBeInTheDocument();
  });

  it('Escape closes only the PAT dialog and leaves key management open', async () => {
    const user = userEvent.setup();
    setupInvokeMocks([mockGitLabRepo]);
    renderHub();

    await user.click(screen.getByRole('button', { name: '来源管理' }));
    await user.click(screen.getByRole('button', { name: '密钥管理' }));
    await user.click(await screen.findByRole('button', { name: '去认证' }));
    await user.keyboard('{Escape}');

    expect(
      screen.queryByRole('dialog', { name: '配置 GitLab 访问密钥' }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole('dialog', { name: '密钥管理' })).toBeInTheDocument();
    expect(screen.getByRole('dialog', { name: '来源管理' })).toBeInTheDocument();
  });

  it('renders hero title, stat pills, and tabs', async () => {
    renderHub();

    expect(await screen.findByRole('heading', { name: 'Skill 中心' })).toBeInTheDocument();
    expect(screen.getByText('3 有效')).toBeInTheDocument();
    expect(screen.getByText('1 无效')).toBeInTheDocument();
    expect(screen.getByText('1 待更新')).toBeInTheDocument();
    expect(screen.getByText('C:\\Users\\dev\\.cursor\\skills')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /已安装 \(4\)/ })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /可安装 \(1\)/ })).toBeInTheDocument();
  });

  it('renders dual-track source tree roots for Skills Sync and iFlytek endpoints', async () => {
    const skillHubEndpoints: SkillHubEndpoint[] = [
      {
        id: 'sync-1',
        name: 'Company Sync',
        baseUrl: 'https://sync.example.com',
        enabled: true,
      },
      {
        id: 'sync-2',
        name: 'Disabled Sync',
        baseUrl: 'https://sync-off.example.com',
        enabled: false,
      },
    ];
    const iflytekEndpoints: IflytekSkillHubEndpoint[] = [
      {
        id: 'iflytek-1',
        name: 'iFlytek Hub',
        baseUrl: 'https://iflytek.example.com',
        enabled: true,
      },
      {
        id: 'iflytek-2',
        name: 'iFlytek Off',
        baseUrl: 'https://iflytek-off.example.com',
        enabled: true,
      },
      {
        id: 'iflytek-3',
        name: 'iFlytek Disabled',
        baseUrl: 'https://iflytek-disabled.example.com',
        enabled: false,
      },
    ];
    renderHub({ skillHubEndpoints, iflytekSkillHubEndpoints: iflytekEndpoints });

    expect(screen.queryByText('Skills Sync 1')).not.toBeInTheDocument();
    expect(screen.queryByText('iFlytek 2')).not.toBeInTheDocument();

    const tree = screen.getByRole('tree');
    expect(within(tree).getByRole('treeitem', { name: /Company Sync/ })).toBeInTheDocument();
    expect(within(tree).getByRole('treeitem', { name: /iFlytek Hub/ })).toBeInTheDocument();
  });

  it('shows installed skills on the installed tab', async () => {
    renderHub();

    expect(await screen.findByText('Explore ideas before implementation.')).toBeInTheDocument();
    expect(screen.getByText('红-绿-重构循环。')).toBeInTheDocument();
    expect(screen.getByText('(无效) broken-skill')).toBeInTheDocument();
    expect(screen.getByText('无效')).toBeInTheDocument();
  });

  it('calls onDeleteMainSkill when delete is clicked', async () => {
    const { onDeleteMainSkill } = renderHub();
    const user = userEvent.setup();

    await screen.findByText('Explore ideas before implementation.');
    const deleteButtons = screen.getAllByRole('button', { name: '删除' });
    await user.click(deleteButtons[0]);

    expect(onDeleteMainSkill).toHaveBeenCalledWith('local/brainstorming', 'brainstorming');
  });

  it('shows update and delete actions for skills with pending updates', async () => {
    renderHub();

    await screen.findByText('红-绿-重构循环。');
    expect(screen.getByRole('button', { name: '更新' })).toBeInTheDocument();
    expect(screen.getByText('有更新')).toBeInTheDocument();
  });

  it('switches to discover tab and shows discoverable skills', async () => {
    const user = userEvent.setup();
    renderHub();

    await screen.findByRole('tab', { name: /可安装 \(1\)/ });
    await user.click(screen.getByRole('tab', { name: /可安装 \(1\)/ }));

    expect(await screen.findByText('PDF 解析与合并。')).toBeInTheDocument();
    const installButtons = screen.getAllByRole('button', { name: '安装' });
    expect(installButtons.some((btn) => btn.classList.contains('btn-sm'))).toBe(true);
    expect(screen.getByRole('button', { name: '刷新列表' })).toBeInTheDocument();
  });

  it('calls onSetMainSkillsDir when edit path button is clicked', async () => {
    const { onSetMainSkillsDir } = renderHub();
    const user = userEvent.setup();

    await screen.findByRole('heading', { name: 'Skill 中心' });
    await user.click(screen.getByRole('button', { name: '更改主库目录' }));

    expect(onSetMainSkillsDir).toHaveBeenCalledTimes(1);
  });

  it('filters installed skills with the updates chip', async () => {
    const user = userEvent.setup();
    renderHub();

    await screen.findByText('Explore ideas before implementation.');
    await user.click(screen.getByRole('button', { name: /有更新 \(1\)/ }));

    expect(screen.queryByText('Explore ideas before implementation.')).not.toBeInTheDocument();
    expect(screen.getByText('红-绿-重构循环。')).toBeInTheDocument();
  });

  it('does not count orphan pending updates that match no installed skill', async () => {
    renderHub({
      pendingUpdates: [
        {
          dirName: 'deleted-skill',
          name: 'Deleted',
          remoteHash: 'zzz',
          storageKey: 'repo/gone/deleted-skill',
        },
      ],
    });

    await screen.findByText('Explore ideas before implementation.');
    expect(screen.queryByText(/待更新/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /有更新 \(0\)/ })).toBeInTheDocument();
  });

  it('shows GitHub source for skills with hub install records', async () => {
    renderHub({ skillRecords: undefined });

    await screen.findByText('红-绿-重构循环。');
    expect(screen.getByText('GitHub')).toBeInTheDocument();
  });

  it('shows GitLab source with host for installed gitlab skills', async () => {
    renderHub({ skillRecords: undefined });

    await screen.findByText('GitLab 来源的 Skill。');
    expect(screen.getByText(/GitLab · gitlab\.example\.com/)).toBeInTheDocument();
  });

  it('filters installed skills by GitLab repo node in source tree', async () => {
    const user = userEvent.setup();
    renderHub({ skillRecords: undefined });

    await screen.findByText('GitLab 来源的 Skill。');
    const gitlabNodeId = repoNodeId('gitlab.example.com', 'team/skills');
    const tree = screen.getByRole('tree');
    const gitlabNode = within(tree).getByRole('treeitem', {
      name: new RegExp('gitlab\\.example\\.com/team/skills'),
    });
    await user.click(gitlabNode);

    expect(screen.getByText('GitLab 来源的 Skill。')).toBeInTheDocument();
    expect(screen.queryByText('Explore ideas before implementation.')).not.toBeInTheDocument();
    expect(screen.queryByText('红-绿-重构循环。')).not.toBeInTheDocument();
    expect(tree.querySelector(`[aria-selected="true"]`)).toBeTruthy();
    expect(gitlabNode.className).toContain('selected');
    expect(gitlabNodeId).toBe('repo:gitlab.example.com/team/skills');
  });

  it('shows GitLab source with host on discover tab', async () => {
    const user = userEvent.setup();
    renderHub({
      discoverSkills: [mockDiscoverable, mockGitlabDiscoverable],
    });

    await screen.findByRole('tab', { name: /可安装 \(2\)/ });
    await user.click(screen.getByRole('tab', { name: /可安装 \(2\)/ }));

    expect(await screen.findByText('GitLab 仓库中的 Skill。')).toBeInTheDocument();
    expect(screen.getByText(/GitLab · gitlab\.example\.com/)).toBeInTheDocument();
  });

  it('filters discover skills by GitLab repo node in source tree', async () => {
    const user = userEvent.setup();
    renderHub({
      discoverSkills: [mockDiscoverable, mockGitlabDiscoverable],
    });

    await screen.findByRole('tab', { name: /可安装 \(2\)/ });
    await user.click(screen.getByRole('tab', { name: /可安装 \(2\)/ }));

    const tree = screen.getByRole('tree');
    await user.click(
      within(tree).getByRole('treeitem', {
        name: new RegExp('gitlab\\.example\\.com/team/skills'),
      }),
    );

    expect(await screen.findByText('GitLab 仓库中的 Skill。')).toBeInTheDocument();
    expect(screen.queryByText('PDF 解析与合并。')).not.toBeInTheDocument();
  });

  it('shows upload button on discover tab when an enabled Hub root is selected', async () => {
    const user = userEvent.setup();
    const hubEndpoint: SkillHubEndpoint = {
      id: 'company-hub',
      name: 'Company Hub',
      baseUrl: 'https://hub.example.com',
      enabled: true,
    };
    renderHub({ skillHubEndpoints: [hubEndpoint] });

    const tree = await screen.findByRole('tree');
    await user.click(within(tree).getByRole('treeitem', { name: /Company Hub/ }));
    await user.click(screen.getByRole('tab', { name: /可安装/ }));

    expect(screen.getByRole('button', { name: '上传到 Hub' })).toBeInTheDocument();
  });

  it('hides upload button when 全部 is selected', async () => {
    const user = userEvent.setup();
    const hubEndpoint: SkillHubEndpoint = {
      id: 'company-hub',
      name: 'Company Hub',
      baseUrl: 'https://hub.example.com',
      enabled: true,
    };
    renderHub({ skillHubEndpoints: [hubEndpoint] });

    const tree = await screen.findByRole('tree');
    await user.click(within(tree).getByRole('treeitem', { name: /Company Hub/ }));
    expect(screen.getByRole('button', { name: '上传到 Hub' })).toBeInTheDocument();

    await user.click(within(tree).getByRole('treeitem', { name: /全部/ }));
    expect(screen.queryByRole('button', { name: '上传到 Hub' })).not.toBeInTheDocument();
  });

  it('calls scanMainLibrary on mount when onRefreshHub is not provided', async () => {
    const onHubSkillsRefresh = vi.fn();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([]);
      if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
      if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
      if (cmd === 'scan_main_library') {
        return Promise.resolve({
          ...mockHubState,
          lastScanAt: '2026-06-30T01:00:00Z',
        });
      }
      return Promise.resolve(null);
    });

    render(
      <SkillHubPage
        mainSkillsDir="C:\\skills"
        hubState={mockHubState}
        discoverSkills={[]}
        pendingUpdates={[]}
        onHubSkillsRefresh={onHubSkillsRefresh}
        onDiscoverSkillsChange={vi.fn()}
        onPendingUpdatesChange={vi.fn()}
        onDeleteMainSkill={vi.fn()}
        onSetMainSkillsDir={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('scan_main_library');
    });
    await waitFor(() => {
      expect(onHubSkillsRefresh).toHaveBeenCalled();
    });
  });

  it('calls onPreviewSkill for installed card title with storageKey', async () => {
    const user = userEvent.setup();
    const onPreviewSkill = vi.fn();
    renderHub({ onPreviewSkill });

    await screen.findByText('Explore ideas before implementation.');
    await user.click(screen.getByRole('button', { name: 'brainstorming' }));

    expect(onPreviewSkill).toHaveBeenCalledWith({
      kind: 'installed',
      storageKey: 'local/brainstorming',
    });
  });

  it('calls onPreviewSkill for discover card title with discoverKey', async () => {
    const user = userEvent.setup();
    const onPreviewSkill = vi.fn();
    renderHub({ onPreviewSkill });

    await user.click(screen.getByRole('tab', { name: /可安装 \(1\)/ }));
    await screen.findByText('PDF 解析与合并。');
    await user.click(screen.getByRole('button', { name: 'PDF Toolkit' }));

    expect(onPreviewSkill).toHaveBeenCalledWith({
      kind: 'discover',
      discoverKey: 'anthropics/skills:skills/pdf-toolkit',
    });
  });

  it('keeps discover card body multi-select when title is clicked separately', async () => {
    const user = userEvent.setup();
    const onPreviewSkill = vi.fn();
    renderHub({ onPreviewSkill });

    await user.click(screen.getByRole('tab', { name: /可安装 \(1\)/ }));
    const desc = await screen.findByText('PDF 解析与合并。');
    await user.click(desc);

    expect(screen.getByText('已选 1 项')).toBeInTheDocument();
    expect(onPreviewSkill).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: 'PDF Toolkit' }));
    expect(onPreviewSkill).toHaveBeenCalledTimes(1);
    expect(screen.getByText('已选 1 项')).toBeInTheDocument();
  });

  it('closes source drawer when opening skill preview', async () => {
    const user = userEvent.setup();
    const onPreviewSkill = vi.fn();
    renderHub({ onPreviewSkill });

    await user.click(screen.getByRole('button', { name: '来源管理' }));
    expect(screen.getByRole('dialog', { name: '来源管理' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'brainstorming' }));

    expect(onPreviewSkill).toHaveBeenCalledWith({
      kind: 'installed',
      storageKey: 'local/brainstorming',
    });
    expect(screen.queryByRole('dialog', { name: '来源管理' })).not.toBeInTheDocument();
  });

  it('calls onCloseSkillPreview when opening source manage drawer', async () => {
    const user = userEvent.setup();
    const onCloseSkillPreview = vi.fn();
    renderHub({ onCloseSkillPreview });

    await user.click(screen.getByRole('button', { name: '来源管理' }));

    expect(onCloseSkillPreview).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('dialog', { name: '来源管理' })).toBeInTheDocument();
  });

  it('opens remote-overwrite confirm when clicking 重新上传 on dirty hub skill', async () => {
    const user = userEvent.setup();
    const onToast = vi.fn();
    const hubState = hubStateWithSkills([hubDirtySkill], {
      [hubDirtyStorageKey]: hubDirtyRecord,
    });
    renderHub({
      hubState,
      pendingUpdates: [],
      onToast,
      onRefreshHub: vi.fn().mockResolvedValue(undefined),
    });

    await screen.findByText('本地已修改的 Hub Skill。');
    await user.click(screen.getByRole('button', { name: '重新上传' }));

    const dialog = screen.getByRole('dialog', { name: '重新上传到 Hub？' });
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveTextContent(/覆盖/);
    expect(dialog).toHaveTextContent(/远程/);
    expect(screen.getByRole('button', { name: '确认覆盖远程' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '确认覆盖远程' }));

    await waitFor(() => {
      expect(uploadSkillToHubMock).toHaveBeenCalledWith(
        'company-hub',
        'tools',
        hubDirtyStorageKey,
      );
    });
    await waitFor(() => {
      expect(onToast).toHaveBeenCalledWith('已重新上传到 Hub');
    });
  });

  it('skips confirm and updates immediately when not localDirty', async () => {
    const user = userEvent.setup();
    const hubState = hubStateWithSkills([hubCleanSkill], {
      [hubCleanStorageKey]: hubCleanRecord,
    });
    renderHub({
      hubState,
      pendingUpdates: [
        {
          dirName: 'clean-skill',
          name: 'Clean Skill',
          currentHash: 'abc',
          remoteHash: 'def',
          storageKey: hubCleanStorageKey,
        },
      ],
    });

    await screen.findByText('未修改的 Hub Skill。');
    await user.click(screen.getByRole('button', { name: '更新' }));

    await waitFor(() => {
      expect(updateSkillMock).toHaveBeenCalledWith(hubCleanStorageKey);
    });
    expect(screen.queryByRole('dialog', { name: '从 Hub 更新到本地？' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '确认覆盖本地' })).not.toBeInTheDocument();
  });

  it('confirms before update when localDirty', async () => {
    const user = userEvent.setup();
    const hubState = hubStateWithSkills([hubDirtySkill], {
      [hubDirtyStorageKey]: hubDirtyRecord,
    });
    renderHub({
      hubState,
      pendingUpdates: [
        {
          dirName: 'dirty-skill',
          name: 'Dirty Skill',
          currentHash: 'abc',
          remoteHash: 'def',
          storageKey: hubDirtyStorageKey,
        },
      ],
    });

    await screen.findByText('本地已修改的 Hub Skill。');
    await user.click(screen.getByRole('button', { name: '更新' }));

    const dialog = screen.getByRole('dialog', { name: '从 Hub 更新到本地？' });
    expect(dialog).toHaveTextContent(/覆盖本地/);
    expect(screen.getByRole('button', { name: '确认覆盖本地' })).toBeInTheDocument();
    expect(updateSkillMock).not.toHaveBeenCalled();

    await user.click(screen.getByRole('button', { name: '确认覆盖本地' }));

    await waitFor(() => {
      expect(updateSkillMock).toHaveBeenCalledWith(hubDirtyStorageKey);
    });
  });

  it('does not upload when reupload confirm is cancelled', async () => {
    const user = userEvent.setup();
    const hubState = hubStateWithSkills([hubDirtySkill], {
      [hubDirtyStorageKey]: hubDirtyRecord,
    });
    renderHub({
      hubState,
      pendingUpdates: [],
    });

    await screen.findByText('本地已修改的 Hub Skill。');
    await user.click(screen.getByRole('button', { name: '重新上传' }));
    expect(screen.getByRole('dialog', { name: '重新上传到 Hub？' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: '取消' }));

    expect(uploadSkillToHubMock).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog', { name: '重新上传到 Hub？' })).not.toBeInTheDocument();
  });

  it('disables confirm dialog buttons while reupload is in flight', async () => {
    const user = userEvent.setup();
    let resolveUpload: () => void = () => {};
    uploadSkillToHubMock.mockImplementationOnce(
      () =>
        new Promise<{ endpoints: []; discoverSkills: [] }>((resolve) => {
          resolveUpload = () => resolve({ endpoints: [], discoverSkills: [] });
        }),
    );
    const hubState = hubStateWithSkills([hubDirtySkill], {
      [hubDirtyStorageKey]: hubDirtyRecord,
    });
    renderHub({
      hubState,
      pendingUpdates: [],
      onRefreshHub: vi.fn().mockResolvedValue(undefined),
    });

    await screen.findByText('本地已修改的 Hub Skill。');
    await user.click(screen.getByRole('button', { name: '重新上传' }));
    await user.click(screen.getByRole('button', { name: '确认覆盖远程' }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: '确认覆盖远程' })).toBeDisabled();
    });

    resolveUpload();
    await waitFor(() => {
      expect(screen.queryByRole('dialog', { name: '重新上传到 Hub？' })).not.toBeInTheDocument();
    });
  });

  it('stops hub group polling and resets selection when selected hub is removed', async () => {
    const user = userEvent.setup();
    const endpoint: SkillHubEndpoint = {
      id: '10-1-1-54-3337',
      name: 'LAN Hub',
      baseUrl: 'http://10.1.1.54:3337',
      enabled: true,
    };

    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([mockGitHubRepo, mockGitLabRepo]);
      if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([endpoint]);
      if (cmd === 'list_iflytek_skill_hub_endpoints') return Promise.resolve([]);
      if (cmd === 'list_gitlab_credentials') return Promise.resolve([]);
      if (cmd === 'list_hub_groups') {
        return Promise.reject({
          message: '无法识别链接格式：10-1-1-54-3337 (Hub 端点不存在: 10-1-1-54-3337)',
        });
      }
      if (cmd === 'scan_main_library') {
        return Promise.resolve({
          ...mockHubState,
          lastScanAt: '2026-06-30T01:00:00Z',
        });
      }
      return Promise.resolve(null);
    });

    const baseProps: ComponentProps<typeof SkillHubPage> = {
      mainSkillsDir: 'C:\\Users\\dev\\.cursor\\skills',
      hubState: mockHubState,
      discoverSkills: [mockDiscoverable],
      pendingUpdates: mockPendingUpdates,
      startupRefreshSettings: {
        github: false,
        gitlab: true,
        skillHub: true,
        iflytekSkillHub: true,
      },
      skillHubEndpoints: [endpoint],
      onHubSkillsRefresh: vi.fn(),
      onDiscoverSkillsChange: vi.fn(),
      onPendingUpdatesChange: vi.fn(),
      onDeleteMainSkill: vi.fn(),
      onSetMainSkillsDir: vi.fn(),
      onRefreshHub: vi.fn().mockResolvedValue(undefined),
      onError: vi.fn(),
    };

    const { rerender } = render(<SkillHubPage {...baseProps} />);

    await user.click(await screen.findByRole('treeitem', { name: /LAN Hub/ }));

    await waitFor(() => {
      expect(
        invokeMock.mock.calls.some(
          (call) =>
            call[0] === 'list_hub_groups' &&
            (call[1] as { hubEndpointId?: string } | undefined)?.hubEndpointId ===
              '10-1-1-54-3337',
        ),
      ).toBe(true);
    });

    const groupCallsWhileSelected = invokeMock.mock.calls.filter(
      (call) => call[0] === 'list_hub_groups',
    ).length;

    const onErrorAfterDelete = vi.fn();
    rerender(
      <SkillHubPage
        {...baseProps}
        skillHubEndpoints={[]}
        onError={onErrorAfterDelete}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByRole('treeitem', { name: /LAN Hub/ })).not.toBeInTheDocument();
      expect(screen.getByRole('treeitem', { name: /全部/ })).toHaveAttribute(
        'aria-selected',
        'true',
      );
    });

    // Unstable onError + empty endpoints must not keep polling the deleted hub.
    await new Promise((resolve) => setTimeout(resolve, 50));
    const groupCallsAfterDelete = invokeMock.mock.calls.filter(
      (call) => call[0] === 'list_hub_groups',
    ).length;
    expect(groupCallsAfterDelete).toBe(groupCallsWhileSelected);
    expect(onErrorAfterDelete).not.toHaveBeenCalled();
  });
});
