import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import MainLibraryPage from '../components/MainLibraryPage';
import type { SkillView } from '../model/types';

const mockSkills: SkillView[] = [
  {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas.',
    path: '/tmp/main-skills/brainstorming',
    valid: true,
    validationErrors: [],
  },
  {
    dirName: 'invalid-skill',
    name: null,
    description: null,
    path: '/tmp/main-skills/invalid-skill',
    valid: false,
    validationErrors: ['Missing skill.yaml'],
  },
];

describe('MainLibraryPage', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders all skills with delete buttons', () => {
    render(
      <MainLibraryPage
        skills={mockSkills}
        validSkillCount={1}
        invalidSkillCount={1}
        onDeleteMainSkill={vi.fn()}
      />
    );

    expect(screen.getByRole('heading', { name: /main library/i })).toBeInTheDocument();
    expect(screen.getByText('1 valid')).toBeInTheDocument();
    expect(screen.getByText('1 invalid')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /all skills \(2\)/i })).toBeInTheDocument();
    expect(screen.getAllByText('brainstorming')[0]).toBeInTheDocument();
    expect(screen.getAllByText('invalid-skill')[0]).toBeInTheDocument();
    expect(screen.getByText('Explore ideas.')).toBeInTheDocument();
    expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();

    const deleteButtons = screen.getAllByRole('button', { name: /delete skill/i });
    expect(deleteButtons).toHaveLength(2);
    expect(deleteButtons[0]).toHaveClass('danger-button');
    expect(deleteButtons[1]).toHaveClass('danger-button');
    expect(deleteButtons[0]).toHaveAttribute('aria-label', 'Delete skill brainstorming');
    expect(deleteButtons[1]).toHaveAttribute('aria-label', 'Delete skill invalid-skill');
    expect(deleteButtons[0]).toHaveAttribute('title', '从主库删除');
    expect(deleteButtons[0]).toHaveTextContent('删除');
    expect(deleteButtons[1]).toHaveTextContent('删除');
  });

  it('calls onDeleteMainSkill with skill dir name when delete button clicked', async () => {
    const onDeleteMainSkill = vi.fn();
    render(
      <MainLibraryPage
        skills={mockSkills}
        validSkillCount={1}
        invalidSkillCount={1}
        onDeleteMainSkill={onDeleteMainSkill}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getAllByRole('button', { name: /delete skill/i })[0]);

    expect(onDeleteMainSkill).toHaveBeenCalledWith('brainstorming');
  });

  it('renders empty state when no skills', () => {
    render(
      <MainLibraryPage
        skills={[]}
        validSkillCount={0}
        invalidSkillCount={0}
        onDeleteMainSkill={vi.fn()}
      />
    );

    expect(screen.getByText('No skills found in the main directory.')).toBeInTheDocument();
    expect(screen.getByText('0 valid')).toBeInTheDocument();
    expect(screen.queryByText(/invalid/)).not.toBeInTheDocument();
  });
});
