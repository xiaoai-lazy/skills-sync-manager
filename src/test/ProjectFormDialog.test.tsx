import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import ProjectFormDialog from '../components/ProjectFormDialog';
import type { AppState, Project } from '../model/types';

const mockAddProject = vi.fn();
const mockUpdateProject = vi.fn();
const mockSelectDirectory = vi.fn();

vi.mock('../api/commands', () => ({
  addProject: (...args: unknown[]) => mockAddProject(...args),
  updateProject: (...args: unknown[]) => mockUpdateProject(...args),
}));

vi.mock('../api/dialog', () => ({
  selectDirectory: (...args: unknown[]) => mockSelectDirectory(...args),
}));

const sampleProject: Project = {
  id: 'project_1',
  name: 'Alpha',
  rootPath: '/tmp/alpha',
  createdAt: '2026-06-23T00:00:00Z',
  updatedAt: '2026-06-23T00:00:00Z',
};

const sampleAppState: AppState = {
  config: {
    version: 5,
    settings: { mainSkillsDir: null, linkStrategy: 'auto' },
    projects: [sampleProject],
    targets: [],
    installations: [],
  },
  skills: [],
  selectedTargetId: null,
  selectedTargetSkills: [],
};

describe('ProjectFormDialog', () => {
  beforeEach(() => {
    mockAddProject.mockReset();
    mockUpdateProject.mockReset();
    mockSelectDirectory.mockReset();
    mockAddProject.mockResolvedValue(sampleAppState);
    mockUpdateProject.mockResolvedValue(sampleAppState);
    mockSelectDirectory.mockResolvedValue(null);
  });

  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <ProjectFormDialog open={false} onClose={vi.fn()} mode="add" onSuccess={vi.fn()} />,
    );

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders add mode with name and root directory picker', () => {
    render(
      <ProjectFormDialog open={true} onClose={vi.fn()} mode="add" onSuccess={vi.fn()} />,
    );

    expect(screen.getByRole('dialog')).toHaveTextContent('添加项目');
    expect(screen.getByLabelText('项目名称')).toBeInTheDocument();
    expect(screen.getByLabelText('项目根目录')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '选择目录' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '添加' })).toBeDisabled();
  });

  it('submits addProject in add mode', async () => {
    const onSuccess = vi.fn();
    const onClose = vi.fn();

    render(
      <ProjectFormDialog
        open={true}
        onClose={onClose}
        mode="add"
        selectedTargetId="target_1"
        onSuccess={onSuccess}
      />,
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('项目名称'), 'Beta');
    mockSelectDirectory.mockResolvedValueOnce('/tmp/beta');
    await user.click(screen.getByRole('button', { name: '选择目录' }));
    await waitFor(() => {
      expect(screen.getByLabelText('项目根目录')).toHaveValue('/tmp/beta');
    });
    await user.click(screen.getByRole('button', { name: '添加' }));

    await waitFor(() => {
      expect(mockAddProject).toHaveBeenCalledWith('Beta', '/tmp/beta', 'target_1');
    });
    expect(onSuccess).toHaveBeenCalledWith(sampleAppState);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('renders edit mode with readonly root path and no directory picker', () => {
    render(
      <ProjectFormDialog
        open={true}
        onClose={vi.fn()}
        mode="edit"
        project={sampleProject}
        onSuccess={vi.fn()}
      />,
    );

    expect(screen.getByRole('dialog')).toHaveTextContent('编辑项目');
    expect(screen.getByLabelText('项目名称')).toHaveValue('Alpha');
    expect(screen.getByLabelText('项目根目录')).toHaveValue('/tmp/alpha');
    expect(screen.getByLabelText('项目根目录')).toHaveAttribute('readonly');
    expect(screen.queryByRole('button', { name: '选择目录' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '保存' })).toBeEnabled();
  });

  it('submits updateProject in edit mode', async () => {
    const onSuccess = vi.fn();
    const onClose = vi.fn();

    render(
      <ProjectFormDialog
        open={true}
        onClose={onClose}
        mode="edit"
        project={sampleProject}
        selectedTargetId="target_1"
        onSuccess={onSuccess}
      />,
    );

    const user = userEvent.setup();
    await user.clear(screen.getByLabelText('项目名称'));
    await user.type(screen.getByLabelText('项目名称'), 'Alpha Renamed');
    await user.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(mockUpdateProject).toHaveBeenCalledWith('project_1', 'Alpha Renamed', 'target_1');
    });
    expect(onSuccess).toHaveBeenCalledWith(sampleAppState);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows error when addProject fails', async () => {
    mockAddProject.mockRejectedValue(new Error('项目根目录无效'));

    render(
      <ProjectFormDialog open={true} onClose={vi.fn()} mode="add" onSuccess={vi.fn()} />,
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('项目名称'), 'Beta');
    mockSelectDirectory.mockResolvedValueOnce('/tmp/beta');
    await user.click(screen.getByRole('button', { name: '选择目录' }));
    await waitFor(() => {
      expect(screen.getByLabelText('项目根目录')).toHaveValue('/tmp/beta');
    });
    await user.click(screen.getByRole('button', { name: '添加' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('项目根目录无效');
    });
  });

  it('shows inline error when directory picker fails in add mode', async () => {
    mockSelectDirectory.mockRejectedValue(new Error('fail'));

    render(
      <ProjectFormDialog open={true} onClose={vi.fn()} mode="add" onSuccess={vi.fn()} />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '选择目录' }));

    await waitFor(() => {
      expect(screen.getByText('目录选择失败，请重试或手动输入路径。')).toBeInTheDocument();
    });
  });

  it('calls onClose when cancel is clicked', async () => {
    const onClose = vi.fn();

    render(
      <ProjectFormDialog open={true} onClose={onClose} mode="add" onSuccess={vi.fn()} />,
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '取消' }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
