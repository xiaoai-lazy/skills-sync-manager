import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useAppUpdater } from '../hooks/useAppUpdater';

vi.mock('../api/updater', () => ({
  checkAppUpdate: vi.fn(),
  installAppUpdate: vi.fn(),
}));

vi.mock('@tauri-apps/api/app', () => ({
  getVersion: vi.fn(),
}));

import { checkAppUpdate, installAppUpdate } from '../api/updater';
import { getVersion } from '@tauri-apps/api/app';

const sampleUpdate = {
  version: '0.8.0',
  currentVersion: '0.7.1',
  notes: 'notes',
};

describe('useAppUpdater', () => {
  beforeEach(() => {
    vi.mocked(getVersion).mockResolvedValue('0.7.1');
    vi.mocked(checkAppUpdate).mockResolvedValue(null);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads app version when enabled', async () => {
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.appVersion).toBe('0.7.1');
    });
  });

  it('startup check sets updateInfo but does not open dialog', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.updateInfo?.version).toBe('0.8.0');
    });
    expect(result.current.updateDialogOpen).toBe(false);
    expect(onToast).not.toHaveBeenCalled();
  });

  it('ignores a second check while in flight', async () => {
    let resolveCheck!: (v: typeof sampleUpdate | null) => void;
    vi.mocked(checkAppUpdate).mockReset();
    vi.mocked(checkAppUpdate).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCheck = resolve;
        })
    );
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    let p1!: Promise<void>;
    let p2!: Promise<void>;
    await act(async () => {
      p1 = result.current.runUpdateCheck('manual');
      p2 = result.current.runUpdateCheck('manual');
    });
    await waitFor(() => {
      expect(result.current.updateChecking).toBe(true);
    });
    await act(async () => {
      resolveCheck(null);
      await p1;
      await p2;
    });

    expect(checkAppUpdate).toHaveBeenCalledTimes(1);
  });

  it('applies only the latest generation result', async () => {
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    vi.mocked(checkAppUpdate).mockResolvedValueOnce(sampleUpdate);
    await act(async () => {
      await result.current.runUpdateCheck('manual');
    });
    expect(result.current.updateInfo?.version).toBe('0.8.0');

    vi.mocked(checkAppUpdate).mockResolvedValueOnce(null);
    await act(async () => {
      await result.current.runUpdateCheck('manual');
    });
    expect(result.current.updateInfo).toBeNull();
    expect(onToast).toHaveBeenCalledWith('已是最新', 'success');
  });

  it('manual failure toasts and keeps updateInfo null', async () => {
    vi.mocked(checkAppUpdate).mockRejectedValue(new Error('network'));
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: false, onToast }));

    await act(async () => {
      await result.current.runUpdateCheck('manual');
    });

    expect(result.current.updateInfo).toBeNull();
    expect(onToast).toHaveBeenCalledWith('检查更新失败', 'error');
  });

  it('openUpdateDialog opens; defer closes but keeps updateInfo', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue(sampleUpdate);
    const onToast = vi.fn();
    const { result } = renderHook(() => useAppUpdater({ enabled: true, onToast }));

    await waitFor(() => {
      expect(result.current.updateInfo).not.toBeNull();
    });

    act(() => {
      result.current.openUpdateDialog();
    });
    expect(result.current.updateDialogOpen).toBe(true);

    act(() => {
      result.current.handleDeferUpdate();
    });
    expect(result.current.updateDialogOpen).toBe(false);
    expect(result.current.updateInfo?.version).toBe('0.8.0');
  });
});
