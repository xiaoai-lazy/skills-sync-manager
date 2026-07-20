import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import SkillCard from '../components/skill-hub/SkillCard';
import { emptyV6SkillViewFields } from '../model/types';
import type { SkillView } from '../model/types';

afterEach(() => cleanup());

function hubSkill(overrides: Partial<SkillView> = {}): SkillView {
  return {
    ...emptyV6SkillViewFields,
    dirName: 'tdd',
    name: 'tdd',
    description: 'desc',
    path: 'C:/skills/hub/e/g/tdd',
    valid: true,
    validationErrors: [],
    storageKey: 'hub/e/g/tdd',
    linkName: 'tdd',
    localDirty: false,
    ...overrides,
  };
}

describe('SkillCard reupload', () => {
  it('shows 已修改 and 重新上传 when localDirty', async () => {
    const onReupload = vi.fn();
    render(
      <SkillCard
        skill={hubSkill({ localDirty: true })}
        mode="installed"
        sourceLabel="Skill Hub · g"
        onReupload={onReupload}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByText('已修改')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '重新上传' }));
    expect(onReupload).toHaveBeenCalledTimes(1);
  });

  it('hides 重新上传 when not localDirty', () => {
    render(
      <SkillCard
        skill={hubSkill({ localDirty: false })}
        mode="installed"
        sourceLabel="Skill Hub · g"
        onReupload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.queryByText('已修改')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '重新上传' })).not.toBeInTheDocument();
  });

  it('shows 更新 and 重新上传 together when hasUpdate and localDirty', () => {
    render(
      <SkillCard
        skill={hubSkill({ localDirty: true })}
        mode="installed"
        hasUpdate
        sourceLabel="Skill Hub · g"
        onUpdate={vi.fn()}
        onReupload={vi.fn()}
        onDelete={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: '更新' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重新上传' })).toBeInTheDocument();
  });
});
