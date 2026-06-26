import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import MainLibrarySummary from '../components/MainLibrarySummary';

describe('MainLibrarySummary', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders directory path and skill counts', () => {
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={2}
        onSetMainSkillsDir={vi.fn()}
        onManageSkills={vi.fn()}
      />
    );

    expect(screen.getByText('/tmp/main-skills')).toBeInTheDocument();
    expect(screen.getByText('10 valid')).toBeInTheDocument();
    expect(screen.getByText('2 invalid')).toBeInTheDocument();
  });

  it('calls onManageSkills when manage button clicked', async () => {
    const onManageSkills = vi.fn();
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={2}
        onSetMainSkillsDir={vi.fn()}
        onManageSkills={onManageSkills}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /manage skills/i }));

    expect(onManageSkills).toHaveBeenCalledTimes(1);
  });

  it('calls onSetMainSkillsDir when change directory button clicked', async () => {
    const onSetMainSkillsDir = vi.fn();
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={2}
        onSetMainSkillsDir={onSetMainSkillsDir}
        onManageSkills={vi.fn()}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /change directory/i }));

    expect(onSetMainSkillsDir).toHaveBeenCalledTimes(1);
  });

  it('does not render invalid count when invalidSkillCount is 0', () => {
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={0}
        onSetMainSkillsDir={vi.fn()}
        onManageSkills={vi.fn()}
      />
    );

    expect(screen.getByText('10 valid')).toBeInTheDocument();
    expect(screen.queryByText(/invalid/)).not.toBeInTheDocument();
  });

  it('calls onSetMainSkillsDir when set directory button clicked', async () => {
    const onSetMainSkillsDir = vi.fn();
    render(
      <MainLibrarySummary
        mainSkillsDir={null}
        validSkillCount={0}
        invalidSkillCount={0}
        onSetMainSkillsDir={onSetMainSkillsDir}
        onManageSkills={vi.fn()}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /set main directory/i }));

    expect(onSetMainSkillsDir).toHaveBeenCalledTimes(1);
  });
});
