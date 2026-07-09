import { useCallback, useEffect, useRef, useState } from 'react';
import type { AppState, MigrationReportDto } from '../model/types';
import { getAppState } from '../api/commands';
import { errorMessage } from '../utils/errorMessage';

function buildMigrationToastMessage(report: MigrationReportDto): string | null {
  if (report.failed.length > 0) {
    return `升级至 v0.6 时部分 Skill 迁移失败（${report.failed.length} 个）`;
  }
  if (report.succeeded.length > 0) {
    let message = `已升级至 v0.6，迁移 ${report.succeeded.length} 个 Skill`;
    if (report.linksRepaired > 0) {
      message += `，修复 ${report.linksRepaired} 条链接`;
    }
    if (report.orphanLocals.length > 0) {
      message += `，${report.orphanLocals.length} 个本地目录待确认`;
    }
    return message;
  }
  return null;
}

export type UseAppBootstrapArgs = {
  appState: AppState | null;
  selectedTargetId: string | null;
  applyRemoteState: (next: AppState) => void;
  syncFromAppState: (state: AppState) => void;
  runBackgroundDiscover: () => Promise<void>;
  runBackgroundCheckUpdates: () => Promise<void>;
};

export function useAppBootstrap({
  appState,
  selectedTargetId,
  applyRemoteState,
  syncFromAppState,
  runBackgroundDiscover,
  runBackgroundCheckUpdates,
}: UseAppBootstrapArgs) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [migrationToast, setMigrationToast] = useState<string | null>(null);
  const [migrationToastIsError, setMigrationToastIsError] = useState(false);
  const migrationToastShownRef = useRef(false);
  const startupBackgroundDone = useRef(false);

  const applyAppStateSuccess = useCallback(
    (next: AppState) => {
      applyRemoteState(next);
      syncFromAppState(next);
      setError(null);
    },
    [applyRemoteState, syncFromAppState]
  );

  const refresh = useCallback(
    async (nextSelectedTargetId: string | null = selectedTargetId): Promise<void> => {
      setLoading(true);
      try {
        const next = await getAppState(nextSelectedTargetId);
        applyRemoteState(next);
        syncFromAppState(next);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setLoading(false);
      }
    },
    [selectedTargetId, applyRemoteState, syncFromAppState]
  );

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!appState?.lastMigrationReport || migrationToastShownRef.current) return;
    migrationToastShownRef.current = true;
    const report = appState.lastMigrationReport;
    const message = buildMigrationToastMessage(report);
    if (!message) return;
    setMigrationToast(message);
    setMigrationToastIsError(report.failed.length > 0);
  }, [appState]);

  useEffect(() => {
    if (!migrationToast) return;
    const timer = window.setTimeout(() => setMigrationToast(null), 8000);
    return () => window.clearTimeout(timer);
  }, [migrationToast]);

  useEffect(() => {
    if (!appState || startupBackgroundDone.current) return;
    startupBackgroundDone.current = true;
    // Serial: avoid parallel load/save races between discover and checkUpdates.
    void (async () => {
      await runBackgroundDiscover();
      await runBackgroundCheckUpdates();
    })();
  }, [appState, runBackgroundDiscover, runBackgroundCheckUpdates]);

  return {
    loading,
    setLoading,
    error,
    setError,
    migrationToast,
    setMigrationToast,
    migrationToastIsError,
    applyAppStateSuccess,
    refresh,
  };
}
