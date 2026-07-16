import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import TargetDetail from '../components/TargetDetail';
import type { SkillWithTargetState, Target } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

const target: Target = {
  id: 'target_1',
  name: 'Claude Global',
  scope: 'global',
  kind: 'custom',
  customPath: '/tmp/target',
  skillsDir: '/tmp/target',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const validSkill: SkillWithTargetState = {
  skill: {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas.',
    path: '/tmp/main-skills/brainstorming',
    valid: true,
    validationErrors: [],
    ...emptyV6SkillViewFields,
    storageKey: 'local/brainstorming',
    linkName: 'brainstorming',
  },
  state: 'notInstalled',
  message: null,
};

const invalidSkill: SkillWithTargetState = {
  skill: {
    dirName: 'invalid-skill',
    name: null,
    description: null,
    path: '/tmp/main-skills/invalid-skill',
    valid: false,
    validationErrors: ['Missing skill.yaml'],
    ...emptyV6SkillViewFields,
    storageKey: 'local/invalid-skill',
    linkName: 'invalid-skill',
  },
  state: 'invalidSkill',
  message: null,
};

describe('TargetDetail', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders Chinese empty state when no target selected', () => {
    render(
      <TargetDetail
        target={null}
        skills={[]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    expect(screen.getByRole('heading', { name: '未选择目标' })).toBeInTheDocument();
    expect(
      screen.getByText('从侧栏选择一个目标目录，以查看和管理 Skill。')
    ).toBeInTheDocument();
  });

  it('renders hero with target name and mono path', () => {
    render(
      <TargetDetail
        target={target}
        skills={[validSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    expect(screen.getByRole('heading', { name: 'Claude Global' })).toBeInTheDocument();
    expect(screen.getByText('/tmp/target')).toBeInTheDocument();
    expect(screen.getByText('Explore ideas.')).toBeInTheDocument();
  });

  it('renders Chinese empty state when no valid skills', () => {
    render(
      <TargetDetail
        target={target}
        skills={[]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    expect(screen.getByText('主库中暂无有效 Skill')).toBeInTheDocument();
  });

  it('renders invalid skills section with disabled install row', () => {
    render(
      <TargetDetail
        target={target}
        skills={[validSkill, invalidSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    expect(screen.getByText('无效 Skill（1）')).toBeInTheDocument();
    expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();

    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes).toHaveLength(2);
    expect(checkboxes[1]).toHaveAttribute('aria-disabled', 'true');
  });

  it('shows sync button when showSyncButton is true and calls onOpenSync', () => {
    const onOpenSync = vi.fn();
    render(
      <TargetDetail
        target={target}
        skills={[validSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
        showSyncButton
        onOpenSync={onOpenSync}
      />,
    );

    const button = screen.getByRole('button', { name: '从其他目录同步…' });
    expect(button).toBeInTheDocument();
    fireEvent.click(button);
    expect(onOpenSync).toHaveBeenCalledTimes(1);
  });

  it('does not show sync button when showSyncButton is absent', () => {
    render(
      <TargetDetail
        target={target}
        skills={[validSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    expect(screen.queryByRole('button', { name: '从其他目录同步…' })).not.toBeInTheDocument();
  });

  it('does not show sync button when showSyncButton is false', () => {
    render(
      <TargetDetail
        target={target}
        skills={[validSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
        showSyncButton={false}
        onOpenSync={() => {}}
      />,
    );

    expect(screen.queryByRole('button', { name: '从其他目录同步…' })).not.toBeInTheDocument();
  });

  it('calls onPreviewSkill from the title and toggles install from the row body', () => {
    const onPreviewSkill = vi.fn();
    const onToggleSkill = vi.fn();
    render(
      <TargetDetail
        target={target}
        skills={[validSkill]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={onToggleSkill}
        onPreviewSkill={onPreviewSkill}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'brainstorming' }));
    expect(onPreviewSkill).toHaveBeenCalledTimes(1);
    expect(onPreviewSkill).toHaveBeenCalledWith('local/brainstorming');
    expect(onToggleSkill).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('checkbox', { name: 'brainstorming 安装状态' }));
    expect(onToggleSkill).toHaveBeenCalledTimes(1);
    expect(onPreviewSkill).toHaveBeenCalledTimes(1);
  });

  it('sorts skills by name and shows source counts', () => {
    const zebraInstalled: SkillWithTargetState = {
      skill: {
        dirName: 'zebra-tools',
        name: 'zebra-tools',
        description: 'Zebra skill.',
        path: '/tmp/main-skills/zebra-tools',
        valid: true,
        validationErrors: [],
        ...emptyV6SkillViewFields,
        storageKey: 'local/zebra-tools',
        linkName: 'zebra-tools',
      },
      state: 'installed',
      message: null,
    };
    const alphaNotInstalled: SkillWithTargetState = {
      skill: {
        dirName: 'alpha-helper',
        name: 'alpha-helper',
        description: 'Alpha skill.',
        path: '/tmp/main-skills/alpha-helper',
        valid: true,
        validationErrors: [],
        ...emptyV6SkillViewFields,
        storageKey: 'local/alpha-helper',
        linkName: 'alpha-helper',
      },
      state: 'notInstalled',
      message: null,
    };

    render(
      <TargetDetail
        target={target}
        skills={[alphaNotInstalled, zebraInstalled]}
        skillRecords={{}}
        endpoints={[]}
        repos={[]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />,
    );

    const zebra = screen.getByText('zebra-tools');
    const alpha = screen.getByText('alpha-helper');
    expect(alpha.compareDocumentPosition(zebra) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    const allNode = screen.getByRole('treeitem', { name: /全部/ });
    expect(allNode).toHaveTextContent('1/2');
  });
});
