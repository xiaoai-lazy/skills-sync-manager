import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import Sidebar from '../components/Sidebar';
import type { Project, Target } from '../model/types';

const globalCustomTarget: Target = {
  id: 'target_global_custom',
  name: 'Custom Global',
  scope: 'global',
  kind: 'custom',
  customPath: '/tmp/global-custom',
  skillsDir: '/tmp/global-custom',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const globalAgentTarget: Target = {
  id: 'target_global_agent',
  name: 'Cursor',
  scope: 'global',
  kind: 'agent',
  agentId: 'cursor',
  skillsDir: '/home/.cursor/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const projectTarget: Target = {
  id: 'target_project_1',
  name: 'Project Target',
  scope: 'project',
  kind: 'custom',
  projectId: 'project_1',
  customPath: '/tmp/project/.cursor/skills',
  skillsDir: '/tmp/project/.cursor/skills',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const sampleProjects: Project[] = [
  {
    id: 'project_1',
    name: 'My App',
    rootPath: '/tmp/project',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
];

const sampleTargets: Target[] = [globalCustomTarget, globalAgentTarget, projectTarget];

function renderSidebar(overrides: Partial<ComponentProps<typeof Sidebar>> = {}) {
  const defaults: ComponentProps<typeof Sidebar> = {
    targets: sampleTargets,
    projects: sampleProjects,
    selectedTargetId: 'target_global_custom',
    selectedProjectId: null,
    expandedProjectIds: new Set<string>(),
    mainView: 'skill-hub',
    onOpenSkillHub: vi.fn(),
    onSelectTarget: vi.fn(),
    onToggleProject: vi.fn(),
    onAddGlobalTarget: vi.fn(),
    onAddProject: vi.fn(),
    onAddProjectTarget: vi.fn(),
    onEditTarget: vi.fn(),
    onEditProject: vi.fn(),
    onDeleteTarget: vi.fn(),
    onDeleteProject: vi.fn(),
  };
  return render(<Sidebar {...defaults} {...overrides} />);
}

describe('Sidebar', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders brand, Skill 中心 nav, Agent and 项目 sections', () => {
    renderSidebar();

    expect(screen.getByText('Skills Sync')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /skill 中心/i })).toBeInTheDocument();
    expect(screen.getByText('Agent')).toBeInTheDocument();
    expect(screen.getByText('项目')).toBeInTheDocument();
  });

  it('shows only global targets under Agent section', () => {
    renderSidebar();

    const agentBlock = screen.getByText('Agent').closest('.sidebar-block') as HTMLElement;
    expect(agentBlock).toBeTruthy();

    const agentList = within(agentBlock).getAllByRole('listitem');
    expect(agentList).toHaveLength(2);
    expect(within(agentBlock).getByText('Custom Global')).toBeInTheDocument();
    expect(within(agentBlock).getByText('Cursor')).toBeInTheDocument();
    expect(within(agentBlock).queryByText('Project Target')).not.toBeInTheDocument();
  });

  it('hides edit button for agent kind targets', () => {
    renderSidebar();

    const agentBlock = screen.getByText('Agent').closest('.sidebar-block') as HTMLElement;
    expect(agentBlock).toBeTruthy();

    const cursorRow = within(agentBlock).getByText('Cursor').closest('.target-item') as HTMLElement;
    expect(cursorRow).toBeTruthy();
    expect(
      within(cursorRow).queryByRole('button', { name: /edit target cursor/i })
    ).not.toBeInTheDocument();
    expect(
      within(cursorRow).getByRole('button', { name: /delete target cursor/i })
    ).toBeInTheDocument();

    const customRow = within(agentBlock).getByText('Custom Global').closest('.target-item') as HTMLElement;
    expect(customRow).toBeTruthy();
    expect(
      within(customRow).getByRole('button', { name: /edit target custom global/i })
    ).toBeInTheDocument();
  });

  it('marks target selected only when mainView is target', () => {
    const { rerender } = renderSidebar({
      mainView: 'skill-hub',
      selectedTargetId: 'target_global_custom',
    });

    let selectedItems = document.querySelectorAll('.target-item.selected');
    expect(selectedItems).toHaveLength(0);

    rerender(
      <Sidebar
        targets={sampleTargets}
        projects={sampleProjects}
        selectedTargetId="target_global_custom"
        selectedProjectId={null}
        expandedProjectIds={new Set()}
        mainView="target"
        onOpenSkillHub={vi.fn()}
        onSelectTarget={vi.fn()}
        onToggleProject={vi.fn()}
        onAddGlobalTarget={vi.fn()}
        onAddProject={vi.fn()}
        onAddProjectTarget={vi.fn()}
        onEditTarget={vi.fn()}
        onEditProject={vi.fn()}
        onDeleteTarget={vi.fn()}
        onDeleteProject={vi.fn()}
      />
    );

    selectedItems = document.querySelectorAll('.target-item.selected');
    expect(selectedItems).toHaveLength(1);
    expect(selectedItems[0]).toHaveTextContent('Custom Global');
  });

  it('calls onOpenSkillHub when Skill 中心 nav clicked', async () => {
    const onOpenSkillHub = vi.fn();
    renderSidebar({ mainView: 'target', onOpenSkillHub });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /skill 中心/i }));

    expect(onOpenSkillHub).toHaveBeenCalledTimes(1);
  });

  it('calls onSelectTarget when a global target is clicked', async () => {
    const onSelectTarget = vi.fn();
    renderSidebar({ onSelectTarget });

    const user = userEvent.setup();
    await user.click(screen.getByText('Cursor'));

    expect(onSelectTarget).toHaveBeenCalledWith('target_global_agent');
  });

  it('calls onAddGlobalTarget when Agent section add button clicked', async () => {
    const onAddGlobalTarget = vi.fn();
    renderSidebar({ onAddGlobalTarget });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add global target/i }));

    expect(onAddGlobalTarget).toHaveBeenCalledTimes(1);
  });

  it('calls onAddProject when 项目 section add button clicked', async () => {
    const onAddProject = vi.fn();
    renderSidebar({ onAddProject });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add project/i }));

    expect(onAddProject).toHaveBeenCalledTimes(1);
  });

  it('renders empty state when no global targets', () => {
    renderSidebar({ targets: [projectTarget] });

    expect(screen.getByText('暂无用户级目标')).toBeInTheDocument();
  });

  it('shows version in the footer', () => {
    renderSidebar({ appVersion: '0.7.1' });
    expect(screen.getByText('v0.7.1')).toBeInTheDocument();
  });

  it('shows update tag when updateAvailable and opens on click', async () => {
    const onOpenUpdate = vi.fn();
    const user = userEvent.setup();
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: true,
      onOpenUpdate,
    });
    expect(screen.queryByRole('button', { name: '检查更新' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '有新版本' }));
    expect(onOpenUpdate).toHaveBeenCalledTimes(1);
  });

  it('shows refresh control when no update and calls onCheckUpdate', async () => {
    const onCheckUpdate = vi.fn();
    const user = userEvent.setup();
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: false,
      updateChecking: false,
      onCheckUpdate,
    });
    await user.click(screen.getByRole('button', { name: '检查更新' }));
    expect(onCheckUpdate).toHaveBeenCalledTimes(1);
  });

  it('disables refresh while updateChecking', () => {
    renderSidebar({
      appVersion: '0.7.1',
      updateAvailable: false,
      updateChecking: true,
      onCheckUpdate: vi.fn(),
    });
    expect(screen.getByRole('button', { name: '检查更新' })).toBeDisabled();
  });
});
