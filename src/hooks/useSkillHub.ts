import { useCallback, useMemo, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { AppState, DiscoverableSkill, SkillUpdateInfo } from '../model/types';
import {
  checkSkillUpdates,
  discoverSkills,
  scanMainLibrary,
} from '../api/skillHub';
import { errorMessage } from '../utils/errorMessage';
import { isInProgressError } from '../utils/ipcError';
import { emptyHubState, hubStateFromAppState } from '../utils/hubStateFromAppState';

type SetAppState = Dispatch<SetStateAction<AppState | null>>;

export function useSkillHub(args: {
  appState: AppState | null;
  setAppState: SetAppState;
  setError: (message: string | null) => void;
}) {
  const { appState, setAppState, setError } = args;

  const [discoverSkillsList, setDiscoverSkillsList] = useState<DiscoverableSkill[]>(
    []
  );
  const [pendingUpdates, setPendingUpdates] = useState<SkillUpdateInfo[]>([]);
  const discoverInFlight = useRef(false);
  const checkInFlight = useRef(false);

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

  const runBackgroundDiscover = useCallback(async (): Promise<void> => {
    if (discoverInFlight.current) return;
    discoverInFlight.current = true;
    try {
      const result = await discoverSkills();
      setDiscoverSkillsList(result.skills);
      if (result.warnings.length > 0 && result.skills.length === 0) {
        setError(result.warnings.join('；'));
      }
    } catch (err) {
      // Startup/background: InProgress is expected under overlap — stay silent.
      if (!isInProgressError(err)) {
        setError(errorMessage(err));
      }
    } finally {
      discoverInFlight.current = false;
    }
  }, [setError]);

  const runBackgroundCheckUpdates = useCallback(async (): Promise<void> => {
    if (checkInFlight.current) return;
    checkInFlight.current = true;
    try {
      const updates = await checkSkillUpdates();
      setPendingUpdates(updates);
      await refreshHub();
    } catch (err) {
      if (!isInProgressError(err)) {
        setError(errorMessage(err));
      }
    } finally {
      checkInFlight.current = false;
    }
  }, [refreshHub, setError]);

  return {
    hubState,
    discoverSkillsList,
    setDiscoverSkillsList,
    pendingUpdates,
    setPendingUpdates,
    syncFromAppState,
    refreshHub,
    runBackgroundDiscover,
    runBackgroundCheckUpdates,
  };
}
