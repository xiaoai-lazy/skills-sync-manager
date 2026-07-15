import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import SyncFromTargetDialog from '../components/SyncFromTargetDialog';
import type { SyncSourceCandidate } from '../utils/targetSyncCandidates';
import type { AppState, SyncTargetInstallationsResponse, Target } from '../model/types';

const destTarget: Target = {
  id: 'dest',
  name: 'Codex',
  scope: 'project',
  kind: 'agent',
  agentId: 'codex',
  projectId: 'project_1',
  skillsDir: '/tmp/project/.codex/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const lowCountTarget: Target = {
  id: 'source_low',
  name: 'Cursor',
  scope: 'project',
  kind: 'agent',
  agentId: 'cursor',
  projectId: 'project_1',
  skillsDir: '/tmp/project/.cursor/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const highCountTarget: Target = {
  id: 'source_high',
  name: 'Claude',
  scope: 'project',
  kind: 'agent',
  agentId: 'claude',
  projectId: 'project_1',
  skillsDir: '/tmp/project/.claude/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const candidates: SyncSourceCandidate[] = [
  { target: highCountTarget, installedCount: 5 },
  { target: lowCountTarget, installedCount: 2 },
];

const emptyState: AppState = {
  config: {
    version: 5,
    settings: {
      mainSkillsDir: null,
      linkStrategy: 'auto',
      startupRefresh: { github: false, gitlab: true, skillHub: true },
    },
    projects: [],
    targets: [],
    installations: [],
  },
  skills: [],
  selectedTargetId: null,
  selectedTargetSkills: [],
  lastMigrationReport: null,
};

function successResponse(
  partial: Partial<SyncTargetInstallationsResponse> = {},
): SyncTargetInstallationsResponse {
  return {
    installed: 0,
    skipped: 0,
    failed: [],
    state: emptyState,
    ...partial,
  };
}

describe('SyncFromTargetDialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('shows post-create title with dest target name', () => {
    render(
      <SyncFromTargetDialog
        open={true}
        mode="post-create"
        destTarget={destTarget}
        candidates={candidates}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(screen.getByRole('dialog')).toHaveTextContent('已添加 Codex');
    expect(screen.getByRole('dialog')).toHaveTextContent('以链接方式安装');
  });

  it('shows manual mode title', () => {
    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(screen.getByRole('dialog')).toHaveTextContent('从项目内其他目录同步');
  });

  it('defaults selection to the candidate with highest installedCount', () => {
    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    const select = screen.getByLabelText('源目录');
    expect(select).toHaveValue('source_high');
    expect(within(select).getByRole('option', { name: /Claude/ })).toHaveTextContent(
      '已装 5',
    );
  });

  it('uses 暂时跳过 in post-create and calls onClose', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <SyncFromTargetDialog
        open={true}
        mode="post-create"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={vi.fn()}
      />,
    );

    await user.click(screen.getByRole('button', { name: '暂时跳过' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('uses 取消 in manual mode and calls onClose', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={vi.fn()}
      />,
    );

    await user.click(screen.getByRole('button', { name: '取消' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onConfirm with selected source and disables controls while pending', async () => {
    let resolveConfirm!: (value: SyncTargetInstallationsResponse) => void;
    const onConfirm = vi.fn(
      () =>
        new Promise<SyncTargetInstallationsResponse>((resolve) => {
          resolveConfirm = resolve;
        }),
    );
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    await user.click(screen.getByRole('button', { name: '同步安装' }));

    expect(onConfirm).toHaveBeenCalledWith('source_high');
    expect(screen.getByRole('button', { name: '同步中…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '取消' })).toBeDisabled();
    expect(screen.getByLabelText('源目录')).toBeDisabled();

    resolveConfirm(successResponse({ installed: 3, skipped: 1 }));

    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
  });

  it('keeps dialog open and shows summary with failed rows when some fail', async () => {
    const onConfirm = vi.fn().mockResolvedValue(
      successResponse({
        installed: 2,
        skipped: 1,
        failed: [
          { storageKey: 'skill-a', label: 'Skill A', error: 'conflict' },
          { storageKey: 'skill-b', label: 'Skill B', error: 'missing' },
        ],
      }),
    );
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    await user.click(screen.getByRole('button', { name: '同步安装' }));

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('已安装 2，跳过 1，失败 2');
    });
    expect(screen.getByText('Skill A: conflict')).toBeInTheDocument();
    expect(screen.getByText('Skill B: missing')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('keeps dialog open with error when entire batch failed', async () => {
    const onConfirm = vi.fn().mockResolvedValue(
      successResponse({
        installed: 0,
        skipped: 0,
        failed: [{ storageKey: 'skill-a', label: 'Skill A', error: 'boom' }],
      }),
    );
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    await user.click(screen.getByRole('button', { name: '同步安装' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('同步未能安装任何 Skill');
    });
    expect(screen.getByRole('status')).toHaveTextContent('已安装 0，跳过 0，失败 1');
    expect(screen.getByText('Skill A: boom')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: '同步安装' })).toBeInTheDocument();
  });

  it('preserves summary and error when candidates array is recreated while open', async () => {
    const onConfirm = vi.fn().mockResolvedValue(
      successResponse({
        installed: 0,
        skipped: 0,
        failed: [{ storageKey: 'skill-a', label: 'Skill A', error: 'boom' }],
      }),
    );
    const onClose = vi.fn();
    const user = userEvent.setup();

    const { rerender } = render(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={candidates}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    await user.click(screen.getByRole('button', { name: '同步安装' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('同步未能安装任何 Skill');
    });
    expect(screen.getByRole('status')).toHaveTextContent('已安装 0，跳过 0，失败 1');

    rerender(
      <SyncFromTargetDialog
        open={true}
        mode="manual"
        destTarget={destTarget}
        candidates={[...candidates]}
        onClose={onClose}
        onConfirm={onConfirm}
      />,
    );

    expect(screen.getByRole('alert')).toHaveTextContent('同步未能安装任何 Skill');
    expect(screen.getByRole('status')).toHaveTextContent('已安装 0，跳过 0，失败 1');
    expect(screen.getByText('Skill A: boom')).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });
});
