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

vi.mock('../api/dialog', () => ({
  selectDirectory: vi.fn(),
}));

import {
  getAppState,
  setMainSkillsDir,
  addTarget,
  updateTarget,
  deleteTarget,
  installSkill,
  uninstallSkill,
  deleteMainSkill,
} from '../api/commands';
import { selectDirectory } from '../api/dialog';

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
    vi.mocked(selectDirectory).mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders main library page by default', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    render(<App />);

    const mainPanel = (await screen.findByRole('main')).closest('.main-panel');
    expect(mainPanel).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: /all skills/i })).toBeInTheDocument();
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

  it('selecting a target from sidebar switches to target detail', async () => {
    const twoTargetState = withTwoTargets(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(twoTargetState);
    render(<App />);

    // Wait for Main Library to render first
    await screen.findByRole('heading', { name: /all skills/i });

    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    expect(targetList).toBeInTheDocument();
    const targetItems = targetList!.querySelectorAll('.target-name');
    expect(targetItems.length).toBe(2);

    // Mock the refresh call after selecting target to return state with target selected
    const selectedState = {
      ...twoTargetState,
      selectedTargetId: 'target_2',
      selectedTargetSkills: [],
    };
    vi.mocked(getAppState).mockResolvedValue(selectedState);

    const user = userEvent.setup();
    await user.click(targetItems[1]!);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Claude Project' })).toBeInTheDocument();
    });
    expect(screen.getByText('No valid skills found in the main library.')).toBeInTheDocument();
  });

  it('clicking manage skills keeps main library view', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    render(<App />);

    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(await screen.findByRole('button', { name: /manage skills/i }));

    expect(screen.getByRole('heading', { name: /all skills/i })).toBeInTheDocument();
  });

  it('notInstalled skill toggle calls install command', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    const installedState = withInstalledSkill(baseAppState);
    vi.mocked(installSkill).mockResolvedValue(installedState);

    render(<App />);

    // Switch to target detail first
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).not.toBeChecked();

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

    // Switch to target detail first
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeChecked();

    await user.click(checkbox);

    await waitFor(() => {
      expect(uninstallSkill).toHaveBeenCalledWith('target_1', 'brainstorming');
    });
  });

  it('conflict state renders disabled controls', async () => {
    const conflictState = withConflictSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(conflictState);

    render(<App />);

    // Switch to target detail first
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('missing state renders disabled controls', async () => {
    const missingState = withMissingSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(missingState);

    render(<App />);

    // Switch to target detail first
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('mismatch state renders disabled controls', async () => {
    const mismatchState = withMismatchSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(mismatchState);

    render(<App />);

    // Switch to target detail first
    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);
    await screen.findByText('Explore ideas.');

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).toBeDisabled();
  });

  it('target detail does not show delete skill button', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    render(<App />);

    const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
    const user = userEvent.setup();
    await user.click(targetList!.querySelector('.target-name')!);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Claude Global' })).toBeInTheDocument();
    });

    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
  });

  it('delete skill button in main library opens confirmation dialog', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const deleteButton = screen.getByRole('button', { name: /delete skill brainstorming/i });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
    expect(screen.getByText(/brainstorming.*will be permanently deleted/)).toBeInTheDocument();
  });

  it('canceling confirmation in main library does not call delete command', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const deleteButton = screen.getByRole('button', { name: /delete skill brainstorming/i });
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

  it('confirming deletion in main library calls delete command with confirmed = true', async () => {
    const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
    vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

    const afterDeleteState = {
      ...baseAppState,
      skills: [],
      selectedTargetSkills: [],
    };
    vi.mocked(deleteMainSkill).mockResolvedValue(afterDeleteState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const deleteButton = screen.getByRole('button', { name: /delete skill brainstorming/i });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();

    const dialog = screen.getByRole('dialog');
    const confirmButton = dialog.querySelector('.danger-button') as HTMLElement;
    await user.click(confirmButton);

    await waitFor(() => {
      expect(deleteMainSkill).toHaveBeenCalledWith('brainstorming', true);
    });
  });

  it('invalid skills are rendered in main library list', async () => {
    const stateWithInvalid = withInvalidSkill(baseAppState);
    vi.mocked(getAppState).mockResolvedValue(stateWithInvalid);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    expect(screen.getAllByText('invalid-skill')[0]).toBeInTheDocument();
    expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();
  });

  it('delete dialog in main library shows link count when skill has installations', async () => {
    const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
    vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const deleteButton = screen.getByRole('button', { name: /delete skill brainstorming/i });
    const user = userEvent.setup();
    await user.click(deleteButton);

    expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
    expect(screen.getByText(/1 recorded target link\(s\) will be removed/)).toBeInTheDocument();
  });

  it('opens PromptDialog and calls setMainSkillsDir when changing directory', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(setMainSkillsDir).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /change directory/i }));

    expect(await screen.findByRole('heading', { name: 'Set Main Skills Directory' })).toBeInTheDocument();

    const input = screen.getByLabelText('Main skills directory path');
    expect(input).toHaveValue('/tmp/main-skills');

    await user.clear(input);
    await user.type(input, '/new/main-skills');
    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(setMainSkillsDir).toHaveBeenCalledWith('/new/main-skills');
    });
  });

  it('opens TargetFormDialog and calls addTarget when adding a target', async () => {
    const stateAfterAdd = {
      ...baseAppState,
      selectedTargetId: 'target_new',
      config: {
        ...baseAppState.config,
        targets: [
          ...baseAppState.config.targets,
          {
            id: 'target_new',
            name: 'New Target',
            skillsDir: '/tmp/new-target',
            createdAt: '2026-06-28T00:00:00Z',
            updatedAt: '2026-06-28T00:00:00Z',
          },
        ],
      },
    };
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(addTarget).mockResolvedValue(stateAfterAdd);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add target/i }));

    expect(await screen.findByRole('heading', { name: 'Add Target' })).toBeInTheDocument();

    await user.type(screen.getByLabelText('Target name'), 'New Target');
    await user.type(screen.getByLabelText('Skills directory path'), '/tmp/new-target');
    await user.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(addTarget).toHaveBeenCalledWith('New Target', '/tmp/new-target');
    });
  });

  it('opens TargetFormDialog prefilled and calls updateTarget when editing a target', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(updateTarget).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /edit target claude global/i }));

    expect(await screen.findByRole('heading', { name: 'Edit Target' })).toBeInTheDocument();
    expect(screen.getByLabelText('Target name')).toHaveValue('Claude Global');
    expect(screen.getByLabelText('Skills directory path')).toHaveValue('/tmp/target');

    const nameInput = screen.getByLabelText('Target name');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Target');

    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(updateTarget).toHaveBeenCalledWith('target_1', 'Updated Target', '/tmp/target');
    });
  });

  it('opens ConfirmDialog and calls deleteTarget when deleting a target', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(deleteTarget).mockResolvedValue(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /delete target claude global/i }));

    expect(await screen.findByRole('heading', { name: 'Delete Target' })).toBeInTheDocument();
    expect(screen.getByText('Delete target "Claude Global"?')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(deleteTarget).toHaveBeenCalledWith('target_1', false);
    });
  });

  it('switches to force delete confirm when deleteTarget fails with recorded installations', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(deleteTarget)
      .mockRejectedValueOnce(new Error('Target has recorded installations'))
      .mockResolvedValueOnce(baseAppState);

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /delete target claude global/i }));

    expect(await screen.findByRole('heading', { name: 'Delete Target' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Delete' }));

    await waitFor(() => {
      expect(deleteTarget).toHaveBeenCalledWith('target_1', false);
    });

    expect(await screen.findByRole('heading', { name: 'Force Delete Target' })).toBeInTheDocument();
    expect(screen.getByText(/has recorded installations/)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Force Delete' }));

    await waitFor(() => {
      expect(deleteTarget).toHaveBeenCalledWith('target_1', true);
    });
  });

  it('main directory change uses picker', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(setMainSkillsDir).mockResolvedValue(baseAppState);
    vi.mocked(selectDirectory).mockResolvedValue('/picked/main-skills');

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /change directory/i }));

    expect(await screen.findByRole('heading', { name: 'Set Main Skills Directory' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    await waitFor(() => {
      expect(selectDirectory).toHaveBeenCalledWith('/tmp/main-skills');
    });

    const input = screen.getByLabelText('Main skills directory path');
    expect(input).toHaveValue('/picked/main-skills');

    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(setMainSkillsDir).toHaveBeenCalledWith('/picked/main-skills');
    });
  });

  it('add target uses picker', async () => {
    const stateAfterAdd = {
      ...baseAppState,
      selectedTargetId: 'target_new',
      config: {
        ...baseAppState.config,
        targets: [
          ...baseAppState.config.targets,
          {
            id: 'target_new',
            name: 'New Target',
            skillsDir: '/picked/new-target',
            createdAt: '2026-06-28T00:00:00Z',
            updatedAt: '2026-06-28T00:00:00Z',
          },
        ],
      },
    };
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(addTarget).mockResolvedValue(stateAfterAdd);
    vi.mocked(selectDirectory).mockResolvedValue('/picked/new-target');

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add target/i }));

    expect(await screen.findByRole('heading', { name: 'Add Target' })).toBeInTheDocument();

    await user.type(screen.getByLabelText('Target name'), 'New Target');
    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    await waitFor(() => {
      expect(selectDirectory).toHaveBeenCalledWith('');
    });

    const skillsDirInput = screen.getByLabelText('Skills directory path');
    expect(skillsDirInput).toHaveValue('/picked/new-target');

    await user.click(screen.getByRole('button', { name: 'Add' }));

    await waitFor(() => {
      expect(addTarget).toHaveBeenCalledWith('New Target', '/picked/new-target');
    });
  });

  it('edit target uses picker', async () => {
    vi.mocked(getAppState).mockResolvedValue(baseAppState);
    vi.mocked(updateTarget).mockResolvedValue(baseAppState);
    vi.mocked(selectDirectory).mockResolvedValue('/picked/target');

    render(<App />);
    await screen.findByRole('heading', { name: /all skills/i });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /edit target claude global/i }));

    expect(await screen.findByRole('heading', { name: 'Edit Target' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    await waitFor(() => {
      expect(selectDirectory).toHaveBeenCalledWith('/tmp/target');
    });

    const skillsDirInput = screen.getByLabelText('Skills directory path');
    expect(skillsDirInput).toHaveValue('/picked/target');

    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(updateTarget).toHaveBeenCalledWith('target_1', 'Claude Global', '/picked/target');
    });
  });
});
