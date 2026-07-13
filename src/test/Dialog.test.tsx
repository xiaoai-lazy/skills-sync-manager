import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import Dialog from '../components/Dialog';

describe('Dialog', () => {
  afterEach(() => {
    cleanup();
  });

  it('does not render when open is false', () => {
    render(
      <Dialog open={false} title="Test" actions={<button>Action</button>}>
        Body
      </Dialog>
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('renders title, children, and actions when open is true', () => {
    render(
      <Dialog open={true} title="Test Title" actions={<button>Action</button>}>
        Body content
      </Dialog>
    );

    const dialog = screen.getByRole('dialog');
    expect(dialog).toBeInTheDocument();
    expect(screen.getByText('Test Title')).toBeInTheDocument();
    expect(screen.getByText('Body content')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Action' })).toBeInTheDocument();
  });

  it('calls onClose when Escape is pressed', async () => {
    const onClose = vi.fn();
    render(
      <Dialog open={true} title="Test" onClose={onClose} actions={<button>Action</button>}>
        Body
      </Dialog>
    );

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when overlay is clicked', async () => {
    const onClose = vi.fn();
    const { container } = render(
      <Dialog open={true} title="Test" onClose={onClose} actions={<button>Action</button>}>
        Body
      </Dialog>
    );

    const overlay = container.querySelector('.dialog-overlay');
    expect(overlay).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(overlay!);

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not call onClose when dialog content is clicked', async () => {
    const onClose = vi.fn();
    render(
      <Dialog open={true} title="Test" onClose={onClose} actions={<button>Action</button>}>
        Body
      </Dialog>
    );

    const user = userEvent.setup();
    await user.click(screen.getByText('Body'));

    expect(onClose).not.toHaveBeenCalled();
  });

  it('does not close on Escape when onClose is not provided', async () => {
    render(
      <Dialog open={true} title="Test" actions={<button>Action</button>}>
        Body
      </Dialog>
    );

    const user = userEvent.setup();
    await user.keyboard('{Escape}');

    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('focuses the first control even when Escape closing is disabled', async () => {
    render(
      <Dialog
        open={true}
        title="Test"
        closeOnEscape={false}
        actions={<button>Last</button>}
      >
        <button>First</button>
      </Dialog>
    );

    await waitFor(() => expect(screen.getByRole('button', { name: 'First' })).toHaveFocus());
  });

  it('traps Tab and Shift+Tab within the dialog', async () => {
    render(
      <Dialog open={true} title="Test" actions={<button>Last</button>}>
        <button>First</button>
      </Dialog>
    );

    const user = userEvent.setup();
    const first = screen.getByRole('button', { name: 'First' });
    const last = screen.getByRole('button', { name: 'Last' });
    await waitFor(() => expect(first).toHaveFocus());

    last.focus();
    await user.tab();
    expect(first).toHaveFocus();

    await user.tab({ shift: true });
    expect(last).toHaveFocus();
  });

  it('restores focus to the opener after closing', async () => {
    const { rerender } = render(
      <>
        <button>Open dialog</button>
        <Dialog open={false} title="Test" actions={<button>Action</button>}>
          Body
        </Dialog>
      </>
    );
    const opener = screen.getByRole('button', { name: 'Open dialog' });
    opener.focus();

    rerender(
      <>
        <button>Open dialog</button>
        <Dialog open={true} title="Test" actions={<button>Action</button>}>
          Body
        </Dialog>
      </>
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'Action' })).toHaveFocus());

    rerender(
      <>
        <button>Open dialog</button>
        <Dialog open={false} title="Test" actions={<button>Action</button>}>
          Body
        </Dialog>
      </>
    );
    expect(screen.getByRole('button', { name: 'Open dialog' })).toHaveFocus();
  });
});
