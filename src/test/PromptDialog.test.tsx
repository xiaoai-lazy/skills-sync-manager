import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import PromptDialog from '../components/PromptDialog';

describe('PromptDialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <PromptDialog
        open={false}
        title="Set Directory"
        label="Path"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders label and default value', () => {
    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByLabelText('Directory path')).toHaveValue('/tmp/skills');
  });

  it('calls onConfirm with current value when confirm is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        confirmLabel="Save"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    const input = screen.getByLabelText('Directory path');
    await user.clear(input);
    await user.type(input, '/new/path');

    await user.click(screen.getByRole('button', { name: 'Save' }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith('/new/path');
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('calls onCancel and not onConfirm when cancel is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /cancel/i }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('does not close when Escape is pressed', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
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
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
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

  it('fills input when directory picker resolves to a path', async () => {
    const onPickDirectory = vi.fn().mockResolvedValue('/chosen/skills');

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        onPickDirectory={onPickDirectory}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const user = userEvent.setup();
    const input = screen.getByLabelText('Directory path');
    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    expect(onPickDirectory).toHaveBeenCalledTimes(1);
    expect(onPickDirectory).toHaveBeenCalledWith('/tmp/skills');
    expect(input).toHaveValue('/chosen/skills');
  });

  it('keeps input unchanged when directory picker resolves null', async () => {
    const onPickDirectory = vi.fn().mockResolvedValue(null);

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        onPickDirectory={onPickDirectory}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const user = userEvent.setup();
    const input = screen.getByLabelText('Directory path');
    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    expect(onPickDirectory).toHaveBeenCalledTimes(1);
    expect(onPickDirectory).toHaveBeenCalledWith('/tmp/skills');
    expect(input).toHaveValue('/tmp/skills');
  });

  it('shows inline error and keeps input unchanged when directory picker rejects', async () => {
    const onPickDirectory = vi.fn().mockRejectedValue(new Error('failed'));

    render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        onPickDirectory={onPickDirectory}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const user = userEvent.setup();
    const input = screen.getByLabelText('Directory path');
    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    expect(onPickDirectory).toHaveBeenCalledTimes(1);
    expect(onPickDirectory).toHaveBeenCalledWith('/tmp/skills');
    expect(input).toHaveValue('/tmp/skills');
    expect(
      screen.getByText('Directory selection failed. Try again or enter the path manually.')
    ).toBeInTheDocument();
  });


  it('ignores stale directory picker results after default value resets', async () => {
    let resolvePicker: (value: string | null) => void = () => {};
    const onPickDirectory = vi.fn(
      () => new Promise<string | null>((resolve) => {
        resolvePicker = resolve;
      })
    );

    const { rerender } = render(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/tmp/skills"
        onPickDirectory={onPickDirectory}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    const user = userEvent.setup();
    const input = screen.getByLabelText('Directory path');
    await user.click(screen.getByRole('button', { name: 'Choose Directory' }));

    rerender(
      <PromptDialog
        open={true}
        title="Set Directory"
        label="Directory path"
        defaultValue="/reset/skills"
        onPickDirectory={onPickDirectory}
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    resolvePicker('/stale/skills');

    expect(await screen.findByDisplayValue('/reset/skills')).toBe(input);
  });

});
