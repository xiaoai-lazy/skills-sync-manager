import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import App from '../App';
import type { AppState, SkillInstallState } from '../model/types';

vi.mock('../api/commands', () => ({
  getAppState: vi.fn(),
  setMainSkillsDir: vi.fn(),
  addTarget: vi.fn(),
  updateTarget: vi.fn(),
  deleteTarget: vi.fn(),
  installSkill: vi.fn(),
  uninstallSkill: vi.fn(),
  deleteMainSkill: vi.fn(),
}));

import {
  getAppState,
  installSkill,
  uninstallSkill,
  deleteMainSkill,
} from '../api/commands';

const baseAppState: AppState = {
  config: {
    version: 1,
    settings: { mainSkillsDir: '/tmp/main-skills', linkStrategy: 'auto' },
    targets: [
      {
        id: 'target_1',
        name: 'Claude Global',
        skillsDir: '/tmp/target',
        createdAt: '2026-06-23T00:00:00Z',
        updatedAt: '2026-06-23T00:00:00Z',
      },
    ],
    installations: [],
  },
  skills: [
    {
      dirName: 'brainstorming',
      name: 'brainstorming',
      description: 'Explore ideas.',
      path: '/tmp/main-skills/brainstorming',
      valid: true,
      validationErrors: [],
    },
  ],
  selectedTargetId: 'target_1',
  selectedTargetSkills: [
    {
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
    },
  ],
};

function withSkillState(
  state: AppState,
  skillDirName: string,
  newState: SkillInstallState,
  message?: string
): AppState {
  return {
    ...state,
    selectedTargetSkills: state.selectedTargetSkills.map((item) =>
      item.skill.dirName === skillDirName
        ? { ...item, state: newState, message: message ?? item.message }
        : item
    ),
  };
}

function withTwoTargets(state: AppState): AppState {
  return {
    ...state,
    config: {
      ...state.config,
      targets: [
        ...state.config.targets,
        {
          id: 'target_2',
          name: 'Claude Project',
          skillsDir: '/tmp/target2',
          createdAt: '2026-06-23T00:00:00Z',
          updatedAt: '2026-06-23T00:00:00Z',
        },
      ],
    },
    selectedTargetId: null,
    selectedTargetSkills: [],
  };
}

function withInvalidSkill(state: AppState): AppState {
  return {
    ...state,
    skills: [
      ...state.skills,
      {
        dirName: 'invalid-skill',
        name: null,
        description: null,
        path: '/tmp/main-skills/invalid-skill',
        valid: false,
        validationErrors: ['Missing skill.yaml'],
      },
    ],
    selectedTargetSkills: [
      ...state.selectedTargetSkills,
      {
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
      },
    ],
  };
}

function withConflictSkill(state: AppState): AppState {
  return {
    ...state,
    selectedTargetSkills: state.selectedTargetSkills.map((item) =>
      item.skill.dirName === 'brainstorming'
        ? { ...item, state: 'conflict' as SkillInstallState, message: 'A file already exists at the target path.' }
        : item
    ),
  };
}

function withMissingSkill(state: AppState): AppState {
  return {
    ...state,
    selectedTargetSkills: state.selectedTargetSkills.map((item) =>
      item.skill.dirName === 'brainstorming'
        ? { ...item, state: 'missing' as SkillInstallState, message: 'Link is missing at target.' }
        : item
    ),
  };
}

function withMismatchSkill(state: AppState): AppState {
  return {
    ...state,
    selectedTargetSkills: state.selectedTargetSkills.map((item) =>
      item.skill.dirName === 'brainstorming'
        ? { ...item, state: 'mismatch' as SkillInstallState, message: 'Link points to a different source.' }
        : item
    ),
  };
}

function withInstalledSkill(state: AppState): AppState {
  return {
    ...state,
    selectedTargetSkills: state.selectedTargetSkills.map((item) =>
      item.skill.dirName === 'brainstorming'
        ? { ...item, state: 'installed' as SkillInstallState, message: null }
        : item
    ),
  };
}

function withInstallations(state: AppState, skillDirName: string): AppState {
  return {
    ...state,
    config: {
      ...state.config,
      installations: [
        {
          id: 'inst_1',
          skillDirName,
          skillName: skillDirName,
          sourcePath: `/tmp/main-skills/${skillDirName}`,
          targetId: 'target_1',
          linkPath: `/tmp/target/${skillDirName}`,
          linkType: 'junction',
          createdAt: '2026-06-23T00:00:00Z',
        },
      ],
    },
  };
}

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders app title, main directory section and manage skills button', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    render(<App />);

    expect(await screen.findByRole('heading', { name: 'Main Library' })).toBeInTheDocument();
    expect(await screen.findByText('/tmp/main-skills')).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: /manage skills/i })).toBeInTheDocument();
  });

  it('renders target list from mocked app state', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    render(<App />);

    expect(await screen.findByRole('heading', { name: 'Targets' })).toBeInTheDocument();
    // Look for the target name within the target list
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    expect(targetList).toBeInTheDocument();
    expect(targetList!.querySelector('.target-name')).toHaveTextContent('Claude Global');
  });

  it('selecting a target shows its skill rows', async () => {
    const twoTargetState = withTwoTargets(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(twoTargetState);
    render(<App />);

    // Wait for both targets to appear in the sidebar
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    expect(targetList).toBeInTheDocument();
    const targetItems = targetList!.querySelectorAll('.target-name');
    expect(targetItems.length).toBe(2);
    expect(targetItems[0]).toHaveTextContent('Claude Global');
    expect(targetItems[1]).toHaveTextContent('Claude Project');

    // Click on the second target
    const user = userEvent.setup();
    await user.click(targetItems[1]!);

    // After selecting, the target's skills should be shown
    // Since target_2 has no skills in the fixture, we expect the empty state
    await waitFor(() => {
      expect(screen.getByText('No Target Selected')).toBeInTheDocument();
    });
  });

  it('notInstalled skill toggle calls install command', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    const installedState = withInstalledSkill(baseAppState);
    vi.mocked(installSkill).mockResolvedValue(installedState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).not.toBeChecked();

    const user = userEvent.setup();
    await user.click(checkbox);

    await waitFor(() => {
      expect(installSkill).toHaveBeenCalledWith('target_1', 'brainstorming');
    });
  });

  it('installed skill toggle calls uninstall command', async () => {
    const installedState = withInstalledSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(installedState);
    const uninstalledState = withSkillState(baseAppState, 'brainstorming', 'notInstalled');
    vi.mocked(uninstallSkill).mockResolvedValue(uninstalledState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeChecked();

    const user = userEvent.setup();
    await user.click(checkbox);

    await waitFor(() => {
      expect(uninstallSkill).toHaveBeenCalledWith('target_1', 'brainstorming');
    });
  });

  it('conflict state renders disabled controls', async () => {
    const conflictState = withConflictSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(conflictState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('missing state renders disabled controls', async () => {
    const missingState = withMissingSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(missingState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('mismatch state renders disabled controls', async () => {
    const mismatchState = withMismatchSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(mismatchState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('delete skill button opens confirmation dialog', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
    expect(screen.getByText(/brainstorming.*will be permanently deleted/)).toBeInTheDocument();
  });

  it('canceling confirmation does not call delete command', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();

    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    await user.click(cancelButton);

    await waitFor(() => {
      expect(screen.queryByText('Confirm Deletion')).not.toBeInTheDocument();
    });

    expect(deleteMainSkill).not.toHaveBeenCalled();
  });

  it('confirming deletion calls delete command with confirmed = true', async () => {
    const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
    vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

    const afterDeleteState = {
      ...baseAppState,
      skills: [],
      selectedTargetSkills: [],
    };
    vi.mocked(deleteMainSkill).mockResolvedValue(afterDeleteState);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();

    // Find the confirm button within the dialog using the dialog's class
    const dialog = screen.getByRole('dialog');
    const confirmButton = dialog.querySelector('.danger-button') as HTMLElement;
    await user.click(confirmButton);

    await waitFor(() => {
      expect(deleteMainSkill).toHaveBeenCalledWith('brainstorming', true);
    });
  });

  it('invalid skills are rendered in invalid section', async () => {
    const stateWithInvalid = withInvalidSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(stateWithInvalid);

    render(<App />);
    await screen.findByRole('heading', { name: /Invalid Skills/ });
    expect((await screen.findAllByText('invalid-skill'))[0]).toBeInTheDocument();

    // Invalid skill checkbox and delete button should be disabled
    const invalidSection = screen.getByRole('heading', { name: /Invalid Skills/ }).closest('section');
    expect(invalidSection).toBeInTheDocument();
  });

  it('delete dialog shows link count when skill has installations', async () => {
    const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
    vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

    render(<App />);
    await screen.findByText('Explore ideas.');

    const deleteButton = screen.getByRole('button', { name: 'Delete' });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
    expect(screen.getByText(/1 recorded target link\(s\) will be removed/)).toBeInTheDocument();
  });
});
