import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import AddTargetDialog from '../components/AddTargetDialog';
import type { AgentPreset, AppState, Target } from '../model/types';

const mockListAgentPresets = vi.fn();
const mockAddAgentTarget = vi.fn();
const mockAddCustomTarget = vi.fn();
const mockSelectDirectory = vi.fn();

vi.mock('../api/commands', () => ({
  listAgentPresets: (...args: unknown[]) => mockListAgentPresets(...args),
  addAgentTarget: (...args: unknown[]) => mockAddAgentTarget(...args),
  addCustomTarget: (...args: unknown[]) => mockAddCustomTarget(...args),
}));

vi.mock('../api/dialog', () => ({
  selectDirectory: (...args: unknown[]) => mockSelectDirectory(...args),
}));

const presets: AgentPreset[] = [
  {
    id: 'cursor',
    displayName: 'Cursor',
    globalPath: '~/.cursor/skills',
    projectRelativePath: '.cursor/skills',
    iconUrl: 'asset://cursor.png',
  },
  {
    id: 'claude',
    displayName: 'Claude Code',
    globalPath: '~/.claude/skills',
    projectRelativePath: '.claude/skills',
  },
  {
    id: 'codex',
    displayName: 'Codex',
    globalPath: '~/.codex/skills',
    projectRelativePath: '.codex/skills',
  },
];

const existingGlobalAgent: Target = {
  id: 'target_cursor',
  name: 'Cursor',
  scope: 'global',
  kind: 'agent',
  agentId: 'cursor',
  skillsDir: '/home/.cursor/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const existingProjectAgent: Target = {
  id: 'target_project_cursor',
  name: 'Cursor',
  scope: 'project',
  kind: 'agent',
  agentId: 'cursor',
  projectId: 'project_1',
  skillsDir: '/tmp/project/.cursor/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const sampleAppState: AppState = {
  config: {
    version: 5,
    settings: { mainSkillsDir: null, linkStrategy: 'auto' },
    projects: [],
    targets: [],
    installations: [],
  },
  skills: [],
  selectedTargetId: null,
  selectedTargetSkills: [],
};

describe('AddTargetDialog', () => {
  beforeEach(() => {
    mockListAgentPresets.mockReset();
    mockAddAgentTarget.mockReset();
    mockAddCustomTarget.mockReset();
    mockSelectDirectory.mockReset();
    mockListAgentPresets.mockResolvedValue(presets);
    mockAddAgentTarget.mockResolvedValue(sampleAppState);
    mockAddCustomTarget.mockResolvedValue(sampleAppState);
    mockSelectDirectory.mockResolvedValue(null);
  });

  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <AddTargetDialog
        open={false}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows global title for user-level scope', async () => {
    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent('添加目标（用户级）');
    });
  });

  it('shows project title with project name', async () => {
    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="project"
        projectId="project_1"
        projectName="My App"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent('添加目标 · My App');
    });
  });

  it('loads presets and renders quick-add chips with icon or initial', async () => {
    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockListAgentPresets).toHaveBeenCalledWith('global', undefined);
    });

    expect(screen.getByText('快捷添加')).toBeInTheDocument();
    const cursorChip = screen.getByRole('button', { name: 'Cursor' });
    expect(cursorChip).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Claude Code' })).toBeInTheDocument();
    expect(cursorChip.querySelector('.quick-add-chip-icon')).toHaveAttribute('src', 'asset://cursor.png');
    expect(within(screen.getByRole('button', { name: 'Claude Code' })).getByText('C')).toBeInTheDocument();
  });

  it('hides presets already added in the same scope', async () => {
    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[existingGlobalAgent]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: 'Cursor' })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Claude Code' })).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: 'Codex' })).toBeInTheDocument();
  });

  it('hides quick-add section and divider when all presets are already added', async () => {
    const allAdded: Target[] = [
      existingGlobalAgent,
      {
        ...existingGlobalAgent,
        id: 'target_claude',
        agentId: 'claude',
        name: 'Claude Code',
      },
      {
        ...existingGlobalAgent,
        id: 'target_codex',
        agentId: 'codex',
        name: 'Codex',
      },
    ];

    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={allAdded}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockListAgentPresets).toHaveBeenCalled();
    });

    expect(screen.queryByText('快捷添加')).not.toBeInTheDocument();
    expect(screen.queryByText('或')).not.toBeInTheDocument();
  });

  it('hides quick-add section when no presets are returned', async () => {
    mockListAgentPresets.mockResolvedValue([]);

    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockListAgentPresets).toHaveBeenCalled();
    });

    expect(screen.queryByText('快捷添加')).not.toBeInTheDocument();
    expect(screen.queryByText('或')).not.toBeInTheDocument();
  });

  it('adds agent target when a quick-add chip is clicked', async () => {
    const onSuccess = vi.fn();
    const onClose = vi.fn();

    render(
      <AddTargetDialog
        open={true}
        onClose={onClose}
        scope="project"
        projectId="project_1"
        projectName="My App"
        existingTargets={[existingProjectAgent]}
        selectedTargetId="target_other"
        onSuccess={onSuccess}
      />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /claude code/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /claude code/i }));

    await waitFor(() => {
      expect(mockAddAgentTarget).toHaveBeenCalledWith(
        'project',
        'claude',
        'project_1',
        'target_other',
      );
    });
    expect(onSuccess).toHaveBeenCalledWith(sampleAppState);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('renders custom form without a custom-path section title', async () => {
    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByLabelText('目标名称')).toBeInTheDocument();
    });

    expect(screen.queryByText('自定义路径')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Skill 目录路径')).toHaveAttribute('readonly');
  });

  it('submits custom target with addCustomTarget', async () => {
    const onSuccess = vi.fn();
    const onClose = vi.fn();

    render(
      <AddTargetDialog
        open={true}
        onClose={onClose}
        scope="global"
        existingTargets={[]}
        selectedTargetId="target_1"
        onSuccess={onSuccess}
      />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByLabelText('目标名称')).toBeInTheDocument();
    });

    await user.type(screen.getByLabelText('目标名称'), 'Tools');
    mockSelectDirectory.mockResolvedValueOnce('/tmp/tools');
    await user.click(screen.getByRole('button', { name: '选择目录' }));
    await waitFor(() => {
      expect(screen.getByLabelText('Skill 目录路径')).toHaveValue('/tmp/tools');
    });

    await user.click(screen.getByRole('button', { name: '添加' }));

    await waitFor(() => {
      expect(mockAddCustomTarget).toHaveBeenCalledWith(
        'global',
        'Tools',
        '/tmp/tools',
        undefined,
        'target_1',
      );
    });
    expect(onSuccess).toHaveBeenCalledWith(sampleAppState);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows error when addCustomTarget fails', async () => {
    mockAddCustomTarget.mockRejectedValue(new Error('目标目录已存在'));

    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByLabelText('目标名称')).toBeInTheDocument();
    });

    await user.type(screen.getByLabelText('目标名称'), 'Tools');
    mockSelectDirectory.mockResolvedValueOnce('/tmp/tools');
    await user.click(screen.getByRole('button', { name: '选择目录' }));
    await waitFor(() => {
      expect(screen.getByLabelText('Skill 目录路径')).toHaveValue('/tmp/tools');
    });
    await user.click(screen.getByRole('button', { name: '添加' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('目标目录已存在');
    });
  });

  it('shows inline error when directory picker fails', async () => {
    mockSelectDirectory.mockRejectedValue(new Error('fail'));

    render(
      <AddTargetDialog
        open={true}
        onClose={vi.fn()}
        scope="global"
        existingTargets={[]}
        onSuccess={vi.fn()}
      />,
    );

    const user = userEvent.setup();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: '选择目录' })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: '选择目录' }));

    await waitFor(() => {
      expect(screen.getByText('目录选择失败，请重试或手动输入路径。')).toBeInTheDocument();
    });
  });
});
