import { useCallback, useEffect, useRef, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import {
  checkAppUpdate,
  installAppUpdate,
  type UpdateInfo,
} from '../api/updater';
import { errorMessage } from '../utils/errorMessage';

export type UpdateCheckSource = 'startup' | 'manual';

export type UseAppUpdaterResult = {
  appVersion: string | null;
  updateInfo: UpdateInfo | null;
  updateDialogOpen: boolean;
  updateInstalling: boolean;
  updateError: string | null;
  updateChecking: boolean;
  runUpdateCheck: (source: UpdateCheckSource) => Promise<void>;
  openUpdateDialog: () => void;
  handleDeferUpdate: () => void;
  handleInstallUpdate: () => Promise<void>;
};

export function useAppUpdater(args: {
  enabled: boolean;
  onToast: (message: string, kind: 'success' | 'error') => void;
}): UseAppUpdaterResult {
  const { enabled, onToast } = args;
  const onToastRef = useRef(onToast);
  onToastRef.current = onToast;

  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);

  const inFlightRef = useRef(false);
  const generationRef = useRef(0);
  const versionLoadedRef = useRef(false);
  const startupStartedRef = useRef(false);

  const runUpdateCheck = useCallback(async (source: UpdateCheckSource) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    const generation = ++generationRef.current;
    setUpdateChecking(true);
    try {
      const info = await checkAppUpdate();
      if (generation !== generationRef.current) return;
      setUpdateInfo(info);
      if (source === 'manual') {
        if (info) {
          // tag appears; no dialog
        } else {
          onToastRef.current('已是最新', 'success');
        }
      }
    } catch {
      if (generation !== generationRef.current) return;
      if (source === 'manual') {
        onToastRef.current('检查更新失败', 'error');
      }
      // startup: silent; leave updateInfo unchanged
    } finally {
      if (generation === generationRef.current) {
        inFlightRef.current = false;
        setUpdateChecking(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!enabled || versionLoadedRef.current) return;
    versionLoadedRef.current = true;
    void getVersion()
      .then((v) => setAppVersion(v))
      .catch(() => {
        /* omit version on failure */
      });
  }, [enabled]);

  useEffect(() => {
    if (!enabled || startupStartedRef.current) return;
    startupStartedRef.current = true;
    void runUpdateCheck('startup');
  }, [enabled, runUpdateCheck]);

  const openUpdateDialog = useCallback(() => {
    setUpdateDialogOpen(true);
  }, []);

  const handleDeferUpdate = useCallback(() => {
    setUpdateDialogOpen(false);
    setUpdateError(null);
  }, []);

  const handleInstallUpdate = useCallback(async () => {
    setUpdateInstalling(true);
    setUpdateError(null);
    try {
      await installAppUpdate();
    } catch (err) {
      setUpdateError(errorMessage(err));
      setUpdateInstalling(false);
    }
  }, []);

  return {
    appVersion,
    updateInfo,
    updateDialogOpen,
    updateInstalling,
    updateError,
    updateChecking,
    runUpdateCheck,
    openUpdateDialog,
    handleDeferUpdate,
    handleInstallUpdate,
  };
}
