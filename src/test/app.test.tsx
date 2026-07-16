import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';

import { render, screen, waitFor, cleanup, within } from '@testing-library/react';

import userEvent from '@testing-library/user-event';

import '@testing-library/jest-dom/vitest';

import App from '../App';

import type { AppState, SkillInstallState } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';



vi.mock('../api/commands', () => ({

  getAppState: vi.fn(),

  setMainSkillsDir: vi.fn(),

  updateTarget: vi.fn(),

  deleteTarget: vi.fn(),

  deleteProject: vi.fn(),

  listAgentPresets: vi.fn(),

  addAgentTarget: vi.fn(),

  addCustomTarget: vi.fn(),

  addProject: vi.fn(),

  updateProject: vi.fn(),

  installSkill: vi.fn(),

  uninstallSkill: vi.fn(),

  deleteMainSkill: vi.fn(),

  syncTargetInstallations: vi.fn(),

}));



vi.mock('../api/skillHub', () => ({

  scanMainLibrary: vi.fn(),

  discoverSkills: vi.fn(),

  checkSkillUpdates: vi.fn(),

  refreshStartupSkillSources: vi.fn(),

  getTargetSkillStates: vi.fn(),

  getSkillRepos: vi.fn().mockResolvedValue([]),

  listSkillHubEndpoints: vi.fn().mockResolvedValue([]),

  addSkillHubEndpoint: vi.fn(),

  listGitlabCredentials: vi.fn().mockResolvedValue([]),

  readSkillMarkdown: vi.fn().mockResolvedValue({
    title: '',
    description: '',
    markdownBody: '',
    origin: 'mainLibrary',
  }),

}));



vi.mock('../api/dialog', () => ({

  selectDirectory: vi.fn(),

}));



vi.mock('../api/updater', () => ({

  checkAppUpdate: vi.fn().mockResolvedValue(null),

  installAppUpdate: vi.fn(),

}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn().mockResolvedValue('0.7.1'),
}));



import {

  getAppState,

  setMainSkillsDir,

  updateTarget,

  deleteTarget,

  listAgentPresets,

  addAgentTarget,

  addCustomTarget,

  installSkill,

  uninstallSkill,

  deleteMainSkill,

  syncTargetInstallations,

} from '../api/commands';

import {

  scanMainLibrary,

  discoverSkills,

  checkSkillUpdates,

  refreshStartupSkillSources,

  getTargetSkillStates,

  addSkillHubEndpoint,

  listSkillHubEndpoints,

} from '../api/skillHub';

import { selectDirectory } from '../api/dialog';
import {
  checkAppUpdate,
} from '../api/updater';



const baseAppState: AppState = {

  config: {

    version: 1,

    settings: {
      mainSkillsDir: '/tmp/main-skills',
      linkStrategy: 'auto',
      startupRefresh: { github: false, gitlab: true, skillHub: true },
    },

    projects: [],

    targets: [

      {

        id: 'target_1',

        name: 'Claude Global',

        scope: 'global',

        kind: 'custom',

        customPath: '/tmp/target',

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

      ...emptyV6SkillViewFields,

      storageKey: 'local/brainstorming',

      linkName: 'brainstorming',

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

        ...emptyV6SkillViewFields,

        storageKey: 'local/brainstorming',

        linkName: 'brainstorming',

      },

      state: 'notInstalled',

      message: null,

    },

  ],

  lastMigrationReport: null,

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

          scope: 'global',

          kind: 'custom',

          customPath: '/tmp/target2',

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

  const invalidSkill = {

    dirName: 'invalid-skill',

    name: null,

    description: null,

    path: '/tmp/main-skills/invalid-skill',

    valid: false,

    validationErrors: ['Missing skill.yaml'],

    ...emptyV6SkillViewFields,

    storageKey: 'local/invalid-skill',

    linkName: 'invalid-skill',

  };

  return {

    ...state,

    skills: [...state.skills, invalidSkill],

    selectedTargetSkills: [

      ...state.selectedTargetSkills,

      {

        skill: invalidSkill,

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



async function getTargetNames(): Promise<HTMLElement[]> {

  const label = await screen.findByText('Agent');

  const section = label.closest('.sidebar-block');

  if (!section) throw new Error('Sidebar target section not found');

  return Array.from(section.querySelectorAll('.target-name'));

}



function withInstallations(state: AppState, skillDirName: string): AppState {
  const storageKey = `local/${skillDirName}`;

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

          skillStorageKey: storageKey,

        },

      ],

    },

  };

}

/** Project with one sibling target; optionally give that sibling installations. */
function withProjectSiblingState(options: {
  siblingInstallCount: number;
}): AppState {
  const siblingId = 'target_project_cursor';
  const installations =
    options.siblingInstallCount > 0
      ? Array.from({ length: options.siblingInstallCount }, (_, i) => ({
          id: `inst_sib_${i}`,
          skillDirName: `skill-${i}`,
          skillName: `skill-${i}`,
          sourcePath: `/tmp/main-skills/skill-${i}`,
          targetId: siblingId,
          linkPath: `/tmp/project/.cursor/skills/skill-${i}`,
          linkType: 'junction' as const,
          createdAt: '2026-06-23T00:00:00Z',
          skillStorageKey: `local/skill-${i}`,
        }))
      : [];

  return {
    ...baseAppState,
    config: {
      ...baseAppState.config,
      projects: [
        {
          id: 'project_1',
          name: 'My App',
          rootPath: '/tmp/project',
          createdAt: '2026-06-23T00:00:00Z',
          updatedAt: '2026-06-23T00:00:00Z',
        },
      ],
      targets: [
        ...baseAppState.config.targets,
        {
          id: siblingId,
          name: 'Cursor',
          scope: 'project',
          kind: 'agent',
          agentId: 'cursor',
          projectId: 'project_1',
          skillsDir: '/tmp/project/.cursor/skills',
          createdAt: '2026-06-23T00:00:00Z',
          updatedAt: '2026-06-23T00:00:00Z',
        },
      ],
      installations,
    },
    selectedTargetId: siblingId,
    selectedTargetSkills: [],
  };
}

function stateAfterAddingProjectClaude(from: AppState): AppState {
  const newTarget = {
    id: 'target_project_claude',
    name: 'Claude Code',
    scope: 'project' as const,
    kind: 'agent' as const,
    agentId: 'claude',
    projectId: 'project_1',
    skillsDir: '/tmp/project/.claude/skills',
    createdAt: '2026-06-28T00:00:00Z',
    updatedAt: '2026-06-28T00:00:00Z',
  };
  return {
    ...from,
    selectedTargetId: newTarget.id,
    selectedTargetSkills: [],
    config: {
      ...from.config,
      targets: [...from.config.targets, newTarget],
    },
  };
}



function setupHubMocks(state: AppState = baseAppState): void {

  vi.mocked(getAppState).mockResolvedValue(state);

  vi.mocked(scanMainLibrary).mockResolvedValue({

    skills: state.skills,

    validCount: state.skills.filter((s) => s.valid).length,

    invalidCount: state.skills.filter((s) => !s.valid).length,

    pendingUpdateCount: 0,

    lastScanAt: '2026-06-30T00:00:00Z',

    skillRecords: state.config.skillRecords ?? {},

  });

  vi.mocked(discoverSkills).mockResolvedValue({ skills: [], warnings: [] });

  vi.mocked(checkSkillUpdates).mockResolvedValue([]);

  vi.mocked(listAgentPresets).mockResolvedValue([]);

  vi.mocked(getTargetSkillStates).mockImplementation(async (targetId) => {

    if (targetId === 'target_1') return state.selectedTargetSkills;

    if (targetId === 'target_2') return [];

    return [];

  });

}



describe('App', () => {

  beforeEach(() => {

    vi.clearAllMocks();

    vi.mocked(selectDirectory).mockReset();

    vi.mocked(checkAppUpdate).mockResolvedValue(null);

    setupHubMocks();

  });



  afterEach(() => {

    cleanup();

  });



  it('renders skill hub view by default', async () => {

    render(<App />);



    const mainPanel = (await screen.findByRole('main')).closest('.main-panel');

    expect(mainPanel).toBeInTheDocument();

    expect(await screen.findByRole('heading', { name: 'Skill 中心' })).toBeInTheDocument();

    expect(screen.getByRole('button', { name: /skill 中心/i })).toHaveClass('active');

  });

  it('shows sidebar version and does not auto-open update dialog when update exists', async () => {
    const { checkAppUpdate } = await import('../api/updater');
    vi.mocked(checkAppUpdate).mockResolvedValue({
      version: '0.8.0',
      currentVersion: '0.7.1',
      notes: 'n',
    });

    render(<App />);

    expect(await screen.findByText('v0.7.1')).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: '有新版本' })).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });


  it('shows a global Toast for source management success', async () => {
    const user = userEvent.setup();
    vi.mocked(addSkillHubEndpoint).mockResolvedValue({ endpoints: [], discoverSkills: [] });
    render(<App />);
    await screen.findByRole('heading', { name: 'Skill 中心' });

    await user.click(screen.getByRole('button', { name: '来源管理' }));
    await user.click(screen.getByRole('button', { name: '添加来源' }));
    const dialog = screen.getByRole('dialog', { name: '添加来源' });
    await user.type(within(dialog).getByLabelText('名称'), 'Company Hub');
    await user.type(within(dialog).getByLabelText('Base URL'), 'https://hub.example.com');
    await user.click(within(dialog).getByRole('button', { name: '添加' }));

    expect(await screen.findByRole('status')).toHaveTextContent('来源已添加');
  });

  it('shows Skill Hub background failures in a fixed error Toast', async () => {
    const user = userEvent.setup();
    vi.mocked(listSkillHubEndpoints).mockRejectedValueOnce(new Error('来源加载失败'));
    render(<App />);
    await screen.findByRole('heading', { name: 'Skill 中心' });

    await user.click(screen.getByRole('button', { name: '来源管理' }));

    const toast = await screen.findByRole('alert');
    expect(toast).toHaveClass('app-toast', 'app-toast--error');
    expect(toast).toHaveTextContent('来源加载失败');
    expect(document.querySelector('.error-banner')).not.toBeInTheDocument();
  });



  it('renders target list from mocked app state', async () => {

    render(<App />);



    expect(await screen.findByText('Agent')).toBeInTheDocument();

    const targetNames = await getTargetNames();

    expect(targetNames).toHaveLength(1);

    expect(targetNames[0]).toHaveTextContent('Claude Global');

  });



  it('selecting a target from sidebar switches to target detail', async () => {

    const twoTargetState = withTwoTargets(baseAppState);

    setupHubMocks(twoTargetState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const targetNames = await getTargetNames();

    expect(targetNames).toHaveLength(2);



    const user = userEvent.setup();

    await user.click(targetNames[1]!);



    await waitFor(() => {

      expect(getTargetSkillStates).toHaveBeenCalledWith('target_2');

    });

    await waitFor(() => {

      expect(screen.getByRole('heading', { name: 'Claude Project' })).toBeInTheDocument();

    });

    expect(screen.getByText('主库中暂无有效 Skill')).toBeInTheDocument();

  });



  it('clicking Skill 中心 nav keeps skill hub view and refreshes hub', async () => {

    render(<App />);



    await screen.findByRole('heading', { name: 'Skill 中心' });

    vi.mocked(scanMainLibrary).mockClear();



    const user = userEvent.setup();

    const targetNames = await getTargetNames();

    await user.click(targetNames[0]!);

    await screen.findByRole('heading', { name: 'Claude Global' });



    await user.click(screen.getByRole('button', { name: /skill 中心/i }));



    expect(screen.getByRole('heading', { name: 'Skill 中心' })).toBeInTheDocument();

    await waitFor(() => {

      expect(scanMainLibrary).toHaveBeenCalled();

    });

  });



  it('notInstalled skill toggle calls install command', async () => {

    const installedState = withInstalledSkill(baseAppState);

    vi.mocked(installSkill).mockResolvedValue(installedState);



    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);

    await screen.findByText('Explore ideas.');



    const checkbox = screen.getByRole('checkbox');

    expect(checkbox).not.toBeChecked();



    await user.click(checkbox);



    await waitFor(() => {

      expect(installSkill).toHaveBeenCalledWith('target_1', 'local/brainstorming');

    });

  });



  it('installed skill toggle calls uninstall command', async () => {

    const installedState = withInstalledSkill(baseAppState);

    setupHubMocks(installedState);

    const uninstalledState = withSkillState(baseAppState, 'brainstorming', 'notInstalled');

    vi.mocked(uninstallSkill).mockResolvedValue(uninstalledState);



    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);

    await screen.findByText('Explore ideas.');



    const checkbox = screen.getByRole('checkbox');

    expect(checkbox).toBeChecked();



    await user.click(checkbox);



    await waitFor(() => {

      expect(uninstallSkill).toHaveBeenCalledWith('target_1', 'local/brainstorming', false);

    });

  });



  it('conflict state renders disabled controls', async () => {

    setupHubMocks(withConflictSkill(baseAppState));

    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);

    await screen.findByText('Explore ideas.');



    const checkbox = screen.getByRole('checkbox');

    expect(checkbox).toHaveAttribute('aria-disabled', 'true');

  });



  it('missing state allows force-clear toggle', async () => {

    setupHubMocks(withMissingSkill(baseAppState));

    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);

    await screen.findByText('Explore ideas.');



    const checkbox = screen.getByRole('checkbox');

    expect(checkbox).toBeEnabled();

    expect(checkbox).toBeChecked();

  });



  it('mismatch state allows force-clear toggle', async () => {

    setupHubMocks(withMismatchSkill(baseAppState));

    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);

    await screen.findByText('Explore ideas.');



    const checkbox = screen.getByRole('checkbox');

    expect(checkbox).toBeEnabled();

    expect(checkbox).toBeChecked();

  });



  it('target detail does not show delete skill button or skill hub navigation', async () => {

    render(<App />);



    const targetNames = await getTargetNames();

    const user = userEvent.setup();

    await user.click(targetNames[0]!);



    await waitFor(() => {

      expect(screen.getByRole('heading', { name: 'Claude Global' })).toBeInTheDocument();

    });



    expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();

    expect(screen.queryByRole('button', { name: /去 skill 中心/i })).not.toBeInTheDocument();

    expect(screen.queryByRole('button', { name: /go to skill hub/i })).not.toBeInTheDocument();

  });



  it('delete skill button in skill hub opens confirmation dialog', async () => {

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const deleteButton = screen.getByRole('button', { name: '删除' });

    const user = userEvent.setup();

    await user.click(deleteButton);



    expect(await screen.findByText('确认删除')).toBeInTheDocument();

    expect(screen.getByText(/brainstorming.*将被永久删除/)).toBeInTheDocument();

  });



  it('canceling confirmation in skill hub does not call delete command', async () => {

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const deleteButton = screen.getByRole('button', { name: '删除' });

    const user = userEvent.setup();

    await user.click(deleteButton);



    expect(await screen.findByText('确认删除')).toBeInTheDocument();



    const cancelButton = screen.getByRole('button', { name: /取消/i });

    await user.click(cancelButton);



    await waitFor(() => {

      expect(screen.queryByText('确认删除')).not.toBeInTheDocument();

    });



    expect(deleteMainSkill).not.toHaveBeenCalled();

  });



  it('confirming deletion in skill hub calls delete command with confirmed = true', async () => {

    const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');

    setupHubMocks(stateWithInstallations);



    const afterDeleteState = {

      ...baseAppState,

      skills: [],

      selectedTargetSkills: [],

    };

    vi.mocked(deleteMainSkill).mockResolvedValue(afterDeleteState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const deleteButton = screen.getByRole('button', { name: '删除' });

    const user = userEvent.setup();

    await user.click(deleteButton);



    expect(await screen.findByText('确认删除')).toBeInTheDocument();



    const dialog = screen.getByRole('dialog');

    const confirmButton = dialog.querySelector('.danger-button') as HTMLElement;

    await user.click(confirmButton);



    await waitFor(() => {

      expect(deleteMainSkill).toHaveBeenCalledWith('local/brainstorming', true);

    });

  });



  it('invalid skills are rendered in skill hub list', async () => {

    setupHubMocks(withInvalidSkill(baseAppState));

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    expect(screen.getByText('(无效) invalid-skill')).toBeInTheDocument();

    expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();

  });



  it('delete dialog in skill hub shows link count when skill has installations', async () => {

    setupHubMocks(withInstallations(baseAppState, 'brainstorming'));

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const deleteButton = screen.getByRole('button', { name: '删除' });

    const user = userEvent.setup();

    await user.click(deleteButton);



    expect(await screen.findByText('确认删除')).toBeInTheDocument();

    expect(screen.getByText(/将先移除 1 条目标链接记录/)).toBeInTheDocument();

  });



  it('opens AddTargetDialog and calls addCustomTarget when adding a target', async () => {

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

            scope: 'global' as const,

            kind: 'custom' as const,

            customPath: '/tmp/new-target',

            skillsDir: '/tmp/new-target',

            createdAt: '2026-06-28T00:00:00Z',

            updatedAt: '2026-06-28T00:00:00Z',

          },

        ],

      },

    };

    vi.mocked(addCustomTarget).mockResolvedValue(stateAfterAdd);

    vi.mocked(getTargetSkillStates).mockResolvedValue([]);

    vi.mocked(selectDirectory).mockResolvedValue('/tmp/new-target');



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /Add global target/i }));



    expect(await screen.findByRole('heading', { name: '添加目标（用户级）' })).toBeInTheDocument();



    await user.type(screen.getByLabelText('目标名称'), 'New Target');

    await user.click(screen.getByRole('button', { name: '选择目录' }));



    await waitFor(() => {

      expect(selectDirectory).toHaveBeenCalledWith('');

      expect(screen.getByLabelText('Skill 目录路径')).toHaveValue('/tmp/new-target');

    });



    await user.click(screen.getByRole('button', { name: '添加' }));



    await waitFor(() => {

      expect(addCustomTarget).toHaveBeenCalledWith(

        'global',

        'New Target',

        '/tmp/new-target',

        undefined,

        'target_1',

      );

    });

  });



  it('opens TargetFormDialog prefilled and calls updateTarget when editing a target', async () => {

    vi.mocked(updateTarget).mockResolvedValue(baseAppState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /edit target claude global/i }));



    expect(await screen.findByRole('heading', { name: '编辑目标' })).toBeInTheDocument();

    expect(screen.getByLabelText('目标名称')).toHaveValue('Claude Global');

    expect(screen.getByLabelText('Skill 目录路径')).toHaveValue('/tmp/target');

    expect(screen.getByLabelText('Skill 目录路径')).toBeDisabled();



    const nameInput = screen.getByLabelText('目标名称');

    await user.clear(nameInput);

    await user.type(nameInput, 'Updated Target');



    await user.click(screen.getByRole('button', { name: '保存' }));



    await waitFor(() => {

      expect(updateTarget).toHaveBeenCalledWith('target_1', 'Updated Target');

    });

  });



  it('opens ConfirmDialog and calls deleteTarget when deleting a target', async () => {

    vi.mocked(deleteTarget).mockResolvedValue(baseAppState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /delete target claude global/i }));



    expect(await screen.findByRole('heading', { name: '删除目标' })).toBeInTheDocument();

    expect(screen.getByText('确定删除目标「Claude Global」吗？')).toBeInTheDocument();



    const deleteDialog = screen.getByRole('dialog');
    const deleteConfirmButton = deleteDialog.querySelector('.danger-button') as HTMLElement;
    await user.click(deleteConfirmButton);



    await waitFor(() => {

      expect(deleteTarget).toHaveBeenCalledWith('target_1', false);

    });

  });



  it('switches to force delete confirm when deleteTarget fails with recorded installations', async () => {

    vi.mocked(deleteTarget)

      .mockRejectedValueOnce(new Error('Target has recorded installations'))

      .mockResolvedValueOnce(baseAppState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /delete target claude global/i }));



    expect(await screen.findByRole('heading', { name: '删除目标' })).toBeInTheDocument();



    const firstDeleteDialog = screen.getByRole('dialog');
    await user.click(firstDeleteDialog.querySelector('.danger-button') as HTMLElement);



    await waitFor(() => {

      expect(deleteTarget).toHaveBeenCalledWith('target_1', false);

    });



    expect(await screen.findByRole('heading', { name: '强制删除目标' })).toBeInTheDocument();

    expect(screen.getByText(/仍有安装记录/)).toBeInTheDocument();



    const forceDeleteDialog = screen.getByRole('dialog');
    await user.click(forceDeleteDialog.querySelector('.danger-button') as HTMLElement);



    await waitFor(() => {

      expect(deleteTarget).toHaveBeenCalledWith('target_1', true);

    });

  });



  it('add target uses directory picker in AddTargetDialog', async () => {

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

            scope: 'global' as const,

            kind: 'custom' as const,

            customPath: '/picked/new-target',

            skillsDir: '/picked/new-target',

            createdAt: '2026-06-28T00:00:00Z',

            updatedAt: '2026-06-28T00:00:00Z',

          },

        ],

      },

    };

    vi.mocked(addCustomTarget).mockResolvedValue(stateAfterAdd);

    vi.mocked(getTargetSkillStates).mockResolvedValue([]);

    vi.mocked(selectDirectory).mockResolvedValue('/picked/new-target');



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /Add global target/i }));



    expect(await screen.findByRole('heading', { name: '添加目标（用户级）' })).toBeInTheDocument();



    await user.type(screen.getByLabelText('目标名称'), 'New Target');

    await user.click(screen.getByRole('button', { name: '选择目录' }));



    await waitFor(() => {

      expect(selectDirectory).toHaveBeenCalledWith('');

    });



    const skillsDirInput = screen.getByLabelText('Skill 目录路径');

    expect(skillsDirInput).toHaveValue('/picked/new-target');



    await user.click(screen.getByRole('button', { name: '添加' }));



    await waitFor(() => {

      expect(addCustomTarget).toHaveBeenCalledWith(

        'global',

        'New Target',

        '/picked/new-target',

        undefined,

        'target_1',

      );

    });

  });



  it('edit custom target keeps skills directory readonly without picker', async () => {

    vi.mocked(updateTarget).mockResolvedValue(baseAppState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: /edit target claude global/i }));



    expect(await screen.findByRole('heading', { name: '编辑目标' })).toBeInTheDocument();



    const skillsDirInput = screen.getByLabelText('Skill 目录路径');

    expect(skillsDirInput).toHaveValue('/tmp/target');

    expect(skillsDirInput).toBeDisabled();

    expect(screen.queryByRole('button', { name: '选择目录' })).not.toBeInTheDocument();



    await user.click(screen.getByRole('button', { name: '保存' }));



    await waitFor(() => {

      expect(updateTarget).toHaveBeenCalledWith('target_1', 'Claude Global');

    });

  });



  it('set main skills dir opens prompt dialog from skill hub', async () => {

    vi.mocked(setMainSkillsDir).mockResolvedValue(baseAppState);



    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    const user = userEvent.setup();

    await user.click(screen.getByRole('button', { name: '更改主库目录' }));



    expect(await screen.findByRole('heading', { name: '设置主库目录' })).toBeInTheDocument();

    expect(screen.getByLabelText('主库目录路径')).toHaveValue('/tmp/main-skills');

  });



  it('runs the source-aware startup refresh instead of full refresh APIs', async () => {

    vi.mocked(refreshStartupSkillSources).mockResolvedValue({

      discoverSkills: [],

      pendingUpdates: [],

      warnings: [],

    });

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });



    await waitFor(() => {

      expect(refreshStartupSkillSources).toHaveBeenCalledTimes(1);

      expect(discoverSkills).not.toHaveBeenCalled();

      expect(checkSkillUpdates).not.toHaveBeenCalled();

    });

  });



  it('keeps startup refresh failures silent', async () => {

    vi.mocked(refreshStartupSkillSources).mockRejectedValue(new Error('internal hub offline'));

    render(<App />);

    await screen.findByRole('heading', { name: 'Skill 中心' });

    await waitFor(() => {

      expect(refreshStartupSkillSources).toHaveBeenCalledTimes(1);

    });

    expect(screen.queryByText('internal hub offline')).not.toBeInTheDocument();

  });

  it('offers post-create sync when new project target has siblings with installations', async () => {
    const initial = withProjectSiblingState({ siblingInstallCount: 2 });
    const afterAdd = stateAfterAddingProjectClaude(initial);
    setupHubMocks(initial);
    vi.mocked(listAgentPresets).mockResolvedValue([
      {
        id: 'claude',
        displayName: 'Claude Code',
        globalPath: '~/.claude/skills',
        projectRelativePath: '.claude/skills',
      },
    ]);
    vi.mocked(addAgentTarget).mockResolvedValue(afterAdd);
    vi.mocked(getTargetSkillStates).mockResolvedValue([]);
    vi.mocked(syncTargetInstallations).mockResolvedValue({
      installed: 2,
      skipped: 0,
      failed: [],
      state: afterAdd,
    });

    render(<App />);
    await screen.findByRole('heading', { name: 'Skill 中心' });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /Add target to My App/i }));

    expect(await screen.findByRole('heading', { name: /添加目标 · My App/ })).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /claude code/i })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /claude code/i }));

    expect(
      await screen.findByRole('heading', { name: '已添加 Claude Code' }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '同步安装' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '暂时跳过' })).toBeInTheDocument();
  });

  it('does not offer post-create sync when project siblings have zero installations', async () => {
    const initial = withProjectSiblingState({ siblingInstallCount: 0 });
    const afterAdd = stateAfterAddingProjectClaude(initial);
    setupHubMocks(initial);
    vi.mocked(listAgentPresets).mockResolvedValue([
      {
        id: 'claude',
        displayName: 'Claude Code',
        globalPath: '~/.claude/skills',
        projectRelativePath: '.claude/skills',
      },
    ]);
    vi.mocked(addAgentTarget).mockResolvedValue(afterAdd);
    vi.mocked(getTargetSkillStates).mockResolvedValue([]);

    render(<App />);
    await screen.findByRole('heading', { name: 'Skill 中心' });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /Add target to My App/i }));

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /claude code/i })).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /claude code/i }));

    await waitFor(() => {
      expect(addAgentTarget).toHaveBeenCalled();
    });

    expect(screen.queryByRole('heading', { name: /已添加/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '同步安装' })).not.toBeInTheDocument();
  });

});

