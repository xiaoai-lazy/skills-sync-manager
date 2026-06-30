import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import SkillHubPage from '../components/skill-hub/SkillHubPage';
import type {
  DiscoverableSkill,
  SkillHubLocalState,
  SkillUpdateInfo,
  SkillView,
} from '../model/types';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const mockSkills: SkillView[] = [
  {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas before implementation.',
    path: 'C:\\skills\\brainstorming',
    valid: true,
    validationErrors: [],
  },
  {
    dirName: 'broken-skill',
    name: null,
    description: null,
    path: 'C:\\skills\\broken-skill',
    valid: false,
    validationErrors: ['缺少 SKILL.md frontmatter'],
  },
  {
    dirName: 'tdd',
    name: 'Test-Driven Development',
    description: '红-绿-重构循环。',
    path: 'C:\\skills\\tdd',
    valid: true,
    validationErrors: [],
  },
  {
    dirName: 'code-review',
    name: 'Code Review',
    description: 'GitLab 来源的 Skill。',
    path: 'C:\\skills\\code-review',
    valid: true,
    validationErrors: [],
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
    },
  },
};

const mockPendingUpdates: SkillUpdateInfo[] = [
  {
    dirName: 'tdd',
    name: 'Test-Driven Development',
    currentHash: 'abc',
    remoteHash: 'def',
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
};

function renderHub(overrides: Partial<ComponentProps<typeof SkillHubPage>> = {}) {
  const onHubStateChange = vi.fn();
  const onDiscoverSkillsChange = vi.fn();
  const onPendingUpdatesChange = vi.fn();
  const onDeleteMainSkill = vi.fn();
  const onSetMainSkillsDir = vi.fn();

  const props = {
    mainSkillsDir: 'C:\\Users\\dev\\.cursor\\skills',
    hubState: mockHubState,
    discoverSkills: [mockDiscoverable],
    pendingUpdates: mockPendingUpdates,
    onHubStateChange,
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

    expect(onDeleteMainSkill).toHaveBeenCalledWith('brainstorming');
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
    expect(screen.getByText(/GitHub · tdd/)).toBeInTheDocument();
  });

  it('shows GitLab source with host for installed gitlab skills', async () => {
    renderHub({ skillRecords: undefined });

    await screen.findByText('GitLab 来源的 Skill。');
    expect(screen.getByText(/GitLab · gitlab\.example\.com · code-review/)).toBeInTheDocument();
  });

  it('filters installed skills by GitLab source filter', async () => {
    const user = userEvent.setup();
    renderHub({ skillRecords: undefined });

    await screen.findByText('GitLab 来源的 Skill。');
    await user.selectOptions(screen.getByLabelText('来源筛选'), 'gitlab');

    expect(screen.getByText('GitLab 来源的 Skill。')).toBeInTheDocument();
    expect(screen.queryByText('Explore ideas before implementation.')).not.toBeInTheDocument();
    expect(screen.queryByText('红-绿-重构循环。')).not.toBeInTheDocument();
  });

  it('shows GitLab source with host on discover tab', async () => {
    const user = userEvent.setup();
    renderHub({
      discoverSkills: [mockDiscoverable, mockGitlabDiscoverable],
    });

    await screen.findByRole('tab', { name: /可安装 \(2\)/ });
    await user.click(screen.getByRole('tab', { name: /可安装 \(2\)/ }));

    expect(await screen.findByText('GitLab 仓库中的 Skill。')).toBeInTheDocument();
    expect(screen.getByText(/GitLab · gitlab\.example\.com · lint-helper/)).toBeInTheDocument();
  });

  it('filters discover skills by GitLab source filter', async () => {
    const user = userEvent.setup();
    renderHub({
      discoverSkills: [mockDiscoverable, mockGitlabDiscoverable],
    });

    await screen.findByRole('tab', { name: /可安装 \(2\)/ });
    await user.click(screen.getByRole('tab', { name: /可安装 \(2\)/ }));
    await user.selectOptions(screen.getByLabelText('来源筛选'), 'gitlab');

    expect(await screen.findByText('GitLab 仓库中的 Skill。')).toBeInTheDocument();
    expect(screen.queryByText('PDF 解析与合并。')).not.toBeInTheDocument();
  });

  it('calls scanMainLibrary on mount when onRefreshHub is not provided', async () => {
    const onHubStateChange = vi.fn();
    invokeMock.mockResolvedValue({
      ...mockHubState,
      lastScanAt: '2026-06-30T01:00:00Z',
      skillRecords: mockHubState.skillRecords,
    });

    render(
      <SkillHubPage
        mainSkillsDir="C:\\skills"
        hubState={mockHubState}
        discoverSkills={[]}
        pendingUpdates={[]}
        onHubStateChange={onHubStateChange}
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
      expect(onHubStateChange).toHaveBeenCalled();
    });
  });
});
