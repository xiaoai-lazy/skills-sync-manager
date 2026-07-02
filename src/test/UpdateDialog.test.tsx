import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import { useEffect, useRef, useState } from 'react';
import UpdateDialog from '../components/UpdateDialog';
import type { UpdateInfo } from '../api/updater';

vi.mock('../api/updater', () => ({
  checkAppUpdate: vi.fn(),
  installAppUpdate: vi.fn(),
}));

import { checkAppUpdate, installAppUpdate } from '../api/updater';

const sampleUpdate: UpdateInfo = {
  version: '0.5.0',
  currentVersion: '0.4.0',
  notes: 'Bug fixes and improvements',
};

describe('UpdateDialog', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('shows version info when open', () => {
    render(
      <UpdateDialog
        open
        update={sampleUpdate}
        installing={false}
        error={null}
        onDefer={vi.fn()}
        onInstall={vi.fn()}
      />
    );

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('0.4.0')).toBeInTheDocument();
    expect(screen.getByText('0.5.0')).toBeInTheDocument();
    expect(screen.getByText('Bug fixes and improvements')).toBeInTheDocument();
  });

  it('calls onDefer when 稍后 is clicked and does not call install', async () => {
    const onDefer = vi.fn();
    const onInstall = vi.fn();
    const user = userEvent.setup();

    render(
      <UpdateDialog
        open
        update={sampleUpdate}
        installing={false}
        error={null}
        onDefer={onDefer}
        onInstall={onInstall}
      />
    );

    await user.click(screen.getByRole('button', { name: '稍后' }));

    expect(onDefer).toHaveBeenCalledTimes(1);
    expect(onInstall).not.toHaveBeenCalled();
    expect(installAppUpdate).not.toHaveBeenCalled();
  });

  it('calls onInstall when 立即更新 is clicked', async () => {
    const onInstall = vi.fn();
    const user = userEvent.setup();

    render(
      <UpdateDialog
        open
        update={sampleUpdate}
        installing={false}
        error={null}
        onDefer={vi.fn()}
        onInstall={onInstall}
      />
    );

    await user.click(screen.getByRole('button', { name: '立即更新' }));

    expect(onInstall).toHaveBeenCalledTimes(1);
  });

  it('disables actions while installing', () => {
    render(
      <UpdateDialog
        open
        update={sampleUpdate}
        installing
        error={null}
        onDefer={vi.fn()}
        onInstall={vi.fn()}
      />
    );

    expect(screen.getByRole('button', { name: '稍后' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '正在更新…' })).toBeDisabled();
  });
});

function StartupUpdateChecker() {
  const dismissedRef = useRef(false);
  const checkStartedRef = useRef(false);
  const [open, setOpen] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);

  useEffect(() => {
    if (dismissedRef.current || checkStartedRef.current) return;
    checkStartedRef.current = true;
    void checkAppUpdate().then((info) => {
      if (info) {
        setUpdate(info);
        setOpen(true);
      }
    });
  }, []);

  return (
    <UpdateDialog
      open={open}
      update={update}
      installing={false}
      error={null}
      onDefer={() => {
        dismissedRef.current = true;
        setOpen(false);
      }}
      onInstall={() => {
        void installAppUpdate();
      }}
    />
  );
}

describe('Startup update check', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('checks once on mount and opens dialog when update is available', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);

    render(<StartupUpdateChecker />);

    await waitFor(() => {
      expect(checkAppUpdate).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('0.5.0')).toBeInTheDocument();
  });

  it('does not re-check after defer in the same session', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);
    const user = userEvent.setup();

    const { rerender } = render(<StartupUpdateChecker />);

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: '稍后' }));

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    rerender(<StartupUpdateChecker />);

    expect(checkAppUpdate).toHaveBeenCalledTimes(1);
  });
});
