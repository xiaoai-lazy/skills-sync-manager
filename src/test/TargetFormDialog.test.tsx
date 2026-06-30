import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import TargetFormDialog from '../components/TargetFormDialog';

describe('TargetFormDialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <TargetFormDialog
        open={false}
        title="Add Target"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders two inputs', () => {
    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText('目标名称')).toBeInTheDocument();
    expect(screen.getByLabelText('Skill 目录路径')).toBeInTheDocument();
  });

  it('disables confirm button when fields are empty or whitespace-only', async () => {
    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        confirmLabel="Add"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const user = userEvent.setup();
    const nameInput = screen.getByLabelText('目标名称');
    const dirInput = screen.getByLabelText('Skill 目录路径');
    const confirmButton = screen.getByRole('button', { name: 'Add' });

    expect(confirmButton).toBeDisabled();

    await user.type(nameInput, 'Claude');
    expect(confirmButton).toBeDisabled();

    await user.type(dirInput, '   ');
    expect(confirmButton).toBeDisabled();

    await user.clear(dirInput);
    await user.type(dirInput, '/tmp/target');
    expect(confirmButton).toBeEnabled();
  });

  it('calls onConfirm with trimmed values when form is submitted', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        confirmLabel="Add"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.type(screen.getByLabelText('目标名称'), '  Claude  ');
    await user.type(screen.getByLabelText('Skill 目录路径'), '  /tmp/target  ');
    await user.click(screen.getByRole('button', { name: 'Add' }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith('Claude', '/tmp/target');
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('prefills initial values in edit mode', () => {
    render(
      <TargetFormDialog
        open={true}
        title="Edit Target"
        initialName="Claude Global"
        initialSkillsDir="/tmp/global"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByLabelText('目标名称')).toHaveValue('Claude Global');
    expect(screen.getByLabelText('Skill 目录路径')).toHaveValue('/tmp/global');
  });

  it('calls onCancel when cancel is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /取消/i }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('does not close when Escape is pressed', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(onCancel).not.toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('does not close when overlay is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    const { container } = render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const overlay = container.querySelector('.dialog-overlay');
    expect(overlay).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(overlay!);

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(onCancel).not.toHaveBeenCalled();
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('fills Skill 目录路径 when onPickDirectory resolves to a string', async () => {
    const onPickDirectory = vi.fn().mockResolvedValue('/chosen/target');
    const user = userEvent.setup();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        initialSkillsDir="/tmp/target"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
        onPickDirectory={onPickDirectory}
      />
    );

    const dirInput = screen.getByLabelText('Skill 目录路径');
    expect(dirInput).toHaveValue('/tmp/target');

    const pickButton = screen.getByRole('button', { name: '选择目录' });
    await user.click(pickButton);

    await waitFor(() => expect(dirInput).toHaveValue('/chosen/target'));
    expect(onPickDirectory).toHaveBeenCalledWith('/tmp/target');
  });

  it('keeps skills directory unchanged when onPickDirectory resolves null', async () => {
    const onPickDirectory = vi.fn().mockResolvedValue(null);
    const user = userEvent.setup();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        initialSkillsDir="/tmp/target"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
        onPickDirectory={onPickDirectory}
      />
    );

    const dirInput = screen.getByLabelText('Skill 目录路径');
    expect(dirInput).toHaveValue('/tmp/target');

    const pickButton = screen.getByRole('button', { name: '选择目录' });
    await user.click(pickButton);

    await waitFor(() => expect(screen.getByRole('button', { name: '选择目录' })).toBeEnabled());
    expect(dirInput).toHaveValue('/tmp/target');
  });

  it('shows inline error and keeps input unchanged when onPickDirectory rejects', async () => {
    const onPickDirectory = vi.fn().mockRejectedValue(new Error('fail'));
    const user = userEvent.setup();

    render(
      <TargetFormDialog
        open={true}
        title="Add Target"
        initialSkillsDir="/tmp/target"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
        onPickDirectory={onPickDirectory}
      />
    );

    const dirInput = screen.getByLabelText('Skill 目录路径');
    expect(dirInput).toHaveValue('/tmp/target');

    const pickButton = screen.getByRole('button', { name: '选择目录' });
    await user.click(pickButton);

    await waitFor(() =>
      expect(
        screen.getByText('目录选择失败，请重试或手动输入路径。')
      ).toBeInTheDocument()
    );
    expect(dirInput).toHaveValue('/tmp/target');
  });
});
