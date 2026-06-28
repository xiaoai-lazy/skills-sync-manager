import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import ConfirmDialog from '../components/ConfirmDialog';

describe('ConfirmDialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <ConfirmDialog
        open={false}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders title, message, and buttons when open', () => {
    render(
      <ConfirmDialog
        open={true}
        title="Confirm Deletion"
        message="Delete this item?"
        confirmLabel="Delete"
        cancelLabel="Cancel"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Confirm Deletion')).toBeInTheDocument();
    expect(screen.getByText('Delete this item?')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
  });

  it('calls onConfirm when confirm button is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Yes' }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('calls onCancel when cancel button is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'No' }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('calls onCancel when Escape is pressed', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('calls onCancel when overlay is clicked', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();

    const { container } = render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={onConfirm}
        onCancel={onCancel}
      />
    );

    const overlay = container.querySelector('.dialog-overlay');
    expect(overlay).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(overlay!);

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it('applies danger-button class when danger is true', () => {
    render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Delete?"
        confirmLabel="Delete"
        cancelLabel="Cancel"
        danger
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(screen.getByRole('button', { name: 'Delete' })).toHaveClass('danger-button');
  });

  it('focuses confirm button when opened', async () => {
    render(
      <ConfirmDialog
        open={true}
        title="Confirm"
        message="Are you sure?"
        confirmLabel="Yes"
        cancelLabel="No"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Yes' })).toHaveFocus();
    });
  });
});
