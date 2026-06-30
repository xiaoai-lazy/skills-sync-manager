import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import TargetDetail from '../components/TargetDetail';
import type { SkillWithTargetState, Target } from '../model/types';

const target: Target = {
  id: 'target_1',
  name: 'Claude Global',
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
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />
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
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />
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
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />
    );

    expect(screen.getByText('主库中暂无有效 Skill')).toBeInTheDocument();
  });

  it('renders invalid skills section with disabled checkbox', () => {
    render(
      <TargetDetail
        target={target}
        skills={[validSkill, invalidSkill]}
        pendingSkillKey={null}
        onToggleSkill={() => {}}
      />
    );

    expect(screen.getByText('无效 Skill（1）')).toBeInTheDocument();
    expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();

    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes).toHaveLength(2);
    expect(checkboxes[1]).toBeDisabled();
  });
});
