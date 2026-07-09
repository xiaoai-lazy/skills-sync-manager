import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import SkillHubPage from '../components/skill-hub/SkillHubPage';
import { repoNodeId } from '../components/skill-hub/sourceTreeUtils';
import type {
  DiscoverableSkill,
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

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

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

function setupInvokeMocks(repos: SkillRepo[] = [mockGitHubRepo, mockGitLabRepo]) {
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === 'get_skill_repos') return Promise.resolve(repos);
    if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
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
  setupInvokeMocks();
});

afterEach(() => {
  cleanup();
});

describe('SkillHubPage', () => {
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

  it('calls scanMainLibrary on mount when onRefreshHub is not provided', async () => {
    const onHubSkillsRefresh = vi.fn();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_skill_repos') return Promise.resolve([]);
      if (cmd === 'list_skill_hub_endpoints') return Promise.resolve([]);
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
});
