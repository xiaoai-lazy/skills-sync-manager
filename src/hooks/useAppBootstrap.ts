import { useCallback, useEffect, useRef, useState } from 'react';
import type { AppState, MigrationReportDto } from '../model/types';
import { getAppState } from '../api/commands';
import { errorMessage } from '../utils/errorMessage';

function buildMigrationToastMessage(report: MigrationReportDto): string | null {
  if (report.failed.length > 0) {
    const repaired =
      report.linksRepaired > 0 ? `，已修复 ${report.linksRepaired} 条链接` : '';
    return `升级/修复时有 ${report.failed.length} 条链接未完成${repaired}`;
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
  if (report.linksRepaired > 0) {
    return `已修复 ${report.linksRepaired} 条目标链接（对齐主库新路径）`;
  }
  return null;
}

export type UseAppBootstrapArgs = {
  appState: AppState | null;
  selectedTargetId: string | null;
  applyRemoteState: (next: AppState) => void;
  syncFromAppState: (state: AppState) => void;
  runStartupRefresh: () => Promise<void>;
};

export function useAppBootstrap({
  appState,
  selectedTargetId,
  applyRemoteState,
  syncFromAppState,
  runStartupRefresh,
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
    void runStartupRefresh();
  }, [appState, runStartupRefresh]);

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
