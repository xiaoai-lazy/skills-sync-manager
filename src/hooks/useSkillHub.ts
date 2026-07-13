import { useCallback, useMemo, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { AppState, DiscoverableSkill, SkillUpdateInfo } from '../model/types';
import { refreshStartupSkillSources, scanMainLibrary } from '../api/skillHub';
import { emptyHubState, hubStateFromAppState } from '../utils/hubStateFromAppState';

type SetAppState = Dispatch<SetStateAction<AppState | null>>;

export function useSkillHub(args: {
  appState: AppState | null;
  setAppState: SetAppState;
  setError: (message: string | null) => void;
}) {
  const { appState, setAppState } = args;

  const [discoverSkillsList, setDiscoverSkillsList] = useState<DiscoverableSkill[]>([]);
  const [pendingUpdates, setPendingUpdates] = useState<SkillUpdateInfo[]>([]);
  const startupRefreshInFlight = useRef(false);

  const hubState = useMemo(
    () => (appState ? hubStateFromAppState(appState) : emptyHubState),
    [appState]
  );

  const syncFromAppState = useCallback((state: AppState) => {
    setDiscoverSkillsList(state.config.skillDiscoverCache?.skills ?? []);
    setPendingUpdates(state.config.skillUpdateCache?.updates ?? []);
  }, []);

  const refreshHub = useCallback(async (): Promise<void> => {
    const next = await scanMainLibrary();
    setAppState((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        skills: next.skills,
        config: {
          ...prev.config,
          skillRecords: next.skillRecords,
        },
      };
    });
  }, [setAppState]);

  const runStartupRefresh = useCallback(async (): Promise<void> => {
    if (startupRefreshInFlight.current) return;
    startupRefreshInFlight.current = true;
    try {
      const result = await refreshStartupSkillSources();
      setDiscoverSkillsList(result.discoverSkills);
      setPendingUpdates(result.pendingUpdates);
    } catch {
      // Startup refresh is best-effort; keep displaying the loaded cache.
    } finally {
      startupRefreshInFlight.current = false;
    }
  }, []);

  return {
    hubState,
    discoverSkillsList,
    setDiscoverSkillsList,
    pendingUpdates,
    setPendingUpdates,
    syncFromAppState,
    refreshHub,
    runStartupRefresh,
  };
}
