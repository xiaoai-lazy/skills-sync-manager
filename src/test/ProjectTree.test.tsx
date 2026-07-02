import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import ProjectTree from '../components/ProjectTree';
import type { Project, Target } from '../model/types';

const projects: Project[] = [
  {
    id: 'project_1',
    name: 'Alpha',
    rootPath: '/tmp/alpha',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
  {
    id: 'project_2',
    name: 'Beta',
    rootPath: '/tmp/beta',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
];

const targets: Target[] = [
  {
    id: 'target_alpha',
    name: 'Alpha Target',
    scope: 'project',
    kind: 'custom',
    projectId: 'project_1',
    customPath: '/tmp/alpha/.cursor/skills',
    skillsDir: '/tmp/alpha/.cursor/skills',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
  {
    id: 'target_beta',
    name: 'Beta Target',
    scope: 'project',
    kind: 'agent',
    projectId: 'project_2',
    agentId: 'cursor',
    skillsDir: '/tmp/beta/.cursor/skills',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
];

function renderProjectTree(overrides: Partial<ComponentProps<typeof ProjectTree>> = {}) {
  const defaults: ComponentProps<typeof ProjectTree> = {
    projects,
    targets,
    expandedProjectIds: new Set(['project_1']),
    selectedProjectId: 'project_1',
    selectedTargetId: null,
    mainView: 'skill-hub',
    onToggleProject: vi.fn(),
    onAddProjectTarget: vi.fn(),
    onEditProject: vi.fn(),
    onDeleteProject: vi.fn(),
    onSelectTarget: vi.fn(),
    onEditTarget: vi.fn(),
    onDeleteTarget: vi.fn(),
  };
  return render(<ProjectTree {...defaults} {...overrides} />);
}

describe('ProjectTree', () => {
  afterEach(() => {
    cleanup();
  });

  it('expands only projects in expandedProjectIds', () => {
    renderProjectTree({ expandedProjectIds: new Set(['project_1']) });

    const visibleChildren = document.querySelectorAll('.project-children.visible');
    expect(visibleChildren).toHaveLength(1);
    expect(visibleChildren[0]).toHaveTextContent('Alpha Target');
    expect(visibleChildren[0]).not.toHaveTextContent('Beta Target');
  });

  it('shows child targets only for expanded projects', () => {
    renderProjectTree({ expandedProjectIds: new Set(['project_2']) });

    const visibleChildren = document.querySelectorAll('.project-children.visible');
    expect(visibleChildren).toHaveLength(1);
    expect(visibleChildren[0]).toHaveTextContent('Beta Target');
    expect(visibleChildren[0]).not.toHaveTextContent('Alpha Target');

    const hiddenAlphaChildren = document.querySelector(
      '.project-node:first-child .project-children'
    );
    expect(hiddenAlphaChildren).not.toHaveClass('visible');
  });

  it('calls onToggleProject when project header clicked', async () => {
    const onToggleProject = vi.fn();
    renderProjectTree({ onToggleProject });

    const user = userEvent.setup();
    await user.click(screen.getByText('Beta'));

    expect(onToggleProject).toHaveBeenCalledWith('project_2');
  });

  it('calls onAddProjectTarget from project header add button', async () => {
    const onAddProjectTarget = vi.fn();
    renderProjectTree({ onAddProjectTarget });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add target to alpha/i }));

    expect(onAddProjectTarget).toHaveBeenCalledWith('project_1');
  });

  it('renders empty state when no projects', () => {
    renderProjectTree({ projects: [] });

    expect(screen.getByText('暂无项目')).toBeInTheDocument();
  });
});
