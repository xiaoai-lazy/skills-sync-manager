import { useCallback, useState } from 'react';
import type { AppState } from '../model/types';
import type { MainView } from '../components/Sidebar';
import { mergeAppState } from '../utils/mergeAppState';

export function useAppState() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [expandedProjectIds, setExpandedProjectIds] = useState<Set<string>>(
    () => new Set()
  );
  const [mainView, setMainView] = useState<MainView>('skill-hub');

  const applyRemoteState = useCallback((next: AppState) => {
    setAppState((prev) => mergeAppState(prev, next));
    setSelectedTargetId(next.selectedTargetId);
  }, []);

  return {
    appState,
    setAppState,
    applyRemoteState,
    selectedTargetId,
    setSelectedTargetId,
    selectedProjectId,
    setSelectedProjectId,
    expandedProjectIds,
    setExpandedProjectIds,
    mainView,
    setMainView,
  };
}
