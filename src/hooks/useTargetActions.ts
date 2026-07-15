import { useCallback, useState, type Dispatch, type SetStateAction } from 'react';
import type {
  AppState,
  SkillInstallState,
  SyncTargetInstallationsResponse,
  Target,
  TargetScope,
} from '../model/types';
import {
  deleteMainSkill,
  deleteTarget,
  installSkill,
  setMainSkillsDir,
  syncTargetInstallations,
  uninstallSkill,
  updateTarget,
} from '../api/commands';
import { getTargetSkillStates } from '../api/skillHub';
import { canForceClearInstallation } from '../components/SkillRow';
import { cleanupWarningsMessage } from '../utils/cleanupWarnings';
import { errorMessage } from '../utils/errorMessage';
import { shouldOfferPostCreateSync } from '../utils/targetSyncCandidates';
import type { MainView } from '../components/Sidebar';

type SetAppState = Dispatch<SetStateAction<AppState | null>>;

export type AddTargetDialogState = {
  open: boolean;
  scope: TargetScope;
  projectId?: string;
  projectName?: string;
};

export type SyncFromTargetDialogState = {
  open: boolean;
  mode: 'post-create' | 'manual';
  destTarget: Target | null;
};

export function useTargetActions(args: {
  appState: AppState | null;
  setAppState: SetAppState;
  applyRemoteState: (next: AppState) => void;
  applyAppStateSuccess: (next: AppState) => void;
  selectedTargetId: string | null;
  setSelectedTargetId: (id: string | null) => void;
  setSelectedProjectId: (id: string | null) => void;
  setExpandedProjectIds: Dispatch<SetStateAction<Set<string>>>;
  setMainView: (view: MainView) => void;
  setLoading: (loading: boolean) => void;
  setError: (message: string | null) => void;
  syncHubFromAppState: (state: AppState) => void;
  refreshHub: () => Promise<void>;
  refresh: (selectedTargetId?: string | null) => Promise<void>;
  setPromptDialogOpen: (open: boolean) => void;
  setPromptDialogDefaultValue: (value: string) => void;
  setAddTargetDialog: Dispatch<SetStateAction<AddTargetDialogState>>;
  setTargetFormOpen: (open: boolean) => void;
  setTargetFormTarget: (target: Target | null) => void;
  deleteTargetData: Target | null;
  setDeleteTargetData: (target: Target | null) => void;
  deleteTargetForce: boolean;
  setDeleteTargetForce: (force: boolean) => void;
  setDeleteTargetConfirmOpen: (open: boolean) => void;
  forceClearSkillKey: string | null;
  setForceClearSkillKey: (key: string | null) => void;
  setForceClearSkillConfirmOpen: (open: boolean) => void;
  deleteSkillStorageKey: string | null;
  setDeleteSkillStorageKey: (key: string | null) => void;
  setDeleteSkillDirName: (name: string | null) => void;
}) {
  const {
    appState,
    setAppState,
    applyRemoteState,
    applyAppStateSuccess,
    selectedTargetId,
    setSelectedTargetId,
    setSelectedProjectId,
    setExpandedProjectIds,
    setMainView,
    setLoading,
    setError,
    syncHubFromAppState,
    refreshHub,
    refresh,
    setPromptDialogOpen,
    setPromptDialogDefaultValue,
    setAddTargetDialog,
    setTargetFormOpen,
    setTargetFormTarget,
    deleteTargetData,
    setDeleteTargetData,
    deleteTargetForce,
    setDeleteTargetForce,
    setDeleteTargetConfirmOpen,
    forceClearSkillKey,
    setForceClearSkillKey,
    setForceClearSkillConfirmOpen,
    deleteSkillStorageKey,
    setDeleteSkillStorageKey,
    setDeleteSkillDirName,
  } = args;

  const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);
  const [syncDialog, setSyncDialog] = useState<SyncFromTargetDialogState>({
    open: false,
    mode: 'post-create',
    destTarget: null,
  });

  const closeSyncDialog = useCallback(() => {
    setSyncDialog((prev) => ({ ...prev, open: false, destTarget: null }));
  }, []);

  const openManualSyncDialog = useCallback((target: Target) => {
    setSyncDialog({ open: true, mode: 'manual', destTarget: target });
  }, []);

  const handleSyncFromTarget = useCallback(
    async (sourceTargetId: string): Promise<SyncTargetInstallationsResponse> => {
      const dest = syncDialog.destTarget;
      if (!dest) {
        throw new Error('No destination target for sync');
      }
      const response = await syncTargetInstallations(sourceTargetId, dest.id);
      applyRemoteState(response.state);
      if (response.failed.length === 0) {
        closeSyncDialog();
      }
      return response;
    },
    [applyRemoteState, closeSyncDialog, syncDialog.destTarget]
  );

  const handleSetMainSkillsDir = useCallback(() => {
    if (!appState) return;
    setPromptDialogDefaultValue(appState.config.settings.mainSkillsDir ?? '');
    setPromptDialogOpen(true);
  }, [appState, setPromptDialogDefaultValue, setPromptDialogOpen]);

  const handleConfirmSetMainSkillsDir = useCallback(
    async (path: string) => {
      setPromptDialogOpen(false);
      setPendingSkillKey('mainDir');
      try {
        const next = await setMainSkillsDir(path);
        applyRemoteState(next);
        syncHubFromAppState(next);
        await refreshHub();
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setPendingSkillKey(null);
      }
    },
    [
      applyRemoteState,
      refreshHub,
      setError,
      setPromptDialogOpen,
      syncHubFromAppState,
    ]
  );

  const handleAddGlobalTarget = useCallback(() => {
    setAddTargetDialog({ open: true, scope: 'global' });
  }, [setAddTargetDialog]);

  const handleAddProjectTarget = useCallback(
    (projectId: string) => {
      const project = appState?.config.projects.find((p) => p.id === projectId);
      setAddTargetDialog({
        open: true,
        scope: 'project',
        projectId,
        projectName: project?.name,
      });
    },
    [appState, setAddTargetDialog]
  );

  const handleAddTargetSuccess = useCallback(
    (next: AppState) => {
      setAddTargetDialog((prev) => ({ ...prev, open: false }));
      applyAppStateSuccess(next);
      setMainView('target');
      const target = next.selectedTargetId
        ? next.config.targets.find((item) => item.id === next.selectedTargetId)
        : undefined;
      if (target?.projectId) {
        setExpandedProjectIds((prev) => new Set(prev).add(target.projectId!));
        setSelectedProjectId(target.projectId);
      }
      if (target && shouldOfferPostCreateSync(next.config, target)) {
        setSyncDialog({ open: true, mode: 'post-create', destTarget: target });
      }
    },
    [
      applyAppStateSuccess,
      setAddTargetDialog,
      setExpandedProjectIds,
      setMainView,
      setSelectedProjectId,
    ]
  );

  const handleEditTarget = useCallback(
    (target: Target) => {
      if (target.kind !== 'custom') return;
      setTargetFormTarget(target);
      setTargetFormOpen(true);
    },
    [setTargetFormOpen, setTargetFormTarget]
  );

  const handleConfirmEditTarget = useCallback(
    async (targetId: string, name: string) => {
      setTargetFormOpen(false);
      setPendingSkillKey(`edit-${targetId}`);
      try {
        const next = await updateTarget(targetId, name);
        applyRemoteState(next);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setPendingSkillKey(null);
      }
    },
    [applyRemoteState, setError, setTargetFormOpen]
  );

  const handleDeleteTarget = useCallback(
    (target: Target) => {
      setDeleteTargetData(target);
      setDeleteTargetForce(false);
      setDeleteTargetConfirmOpen(true);
    },
    [setDeleteTargetConfirmOpen, setDeleteTargetData, setDeleteTargetForce]
  );

  const handleConfirmDeleteTarget = useCallback(async () => {
    if (!deleteTargetData) return;
    const target = deleteTargetData;
    const force = deleteTargetForce;
    setPendingSkillKey(`delete-${target.id}`);
    try {
      const next = await deleteTarget(target.id, force);
      setDeleteTargetConfirmOpen(false);
      setDeleteTargetData(null);
      setDeleteTargetForce(false);
      applyRemoteState(next);
      const warning = cleanupWarningsMessage(next);
      setError(warning);
    } catch (err) {
      const msg = errorMessage(err);
      if (!force) {
        setDeleteTargetForce(true);
      } else {
        setDeleteTargetConfirmOpen(false);
        setDeleteTargetData(null);
        setDeleteTargetForce(false);
        setError(msg);
      }
    } finally {
      setPendingSkillKey(null);
    }
  }, [
    applyRemoteState,
    deleteTargetData,
    deleteTargetForce,
    setDeleteTargetConfirmOpen,
    setDeleteTargetData,
    setDeleteTargetForce,
    setError,
  ]);

  const handleCancelDeleteTarget = useCallback(() => {
    setDeleteTargetConfirmOpen(false);
    setDeleteTargetData(null);
    setDeleteTargetForce(false);
  }, [setDeleteTargetConfirmOpen, setDeleteTargetData, setDeleteTargetForce]);

  const handleSelectTarget = useCallback(
    (targetId: string) => {
      setMainView('target');
      setSelectedTargetId(targetId);
      const target = appState?.config.targets.find((item) => item.id === targetId);
      if (target?.projectId) {
        setSelectedProjectId(target.projectId);
        setExpandedProjectIds((prev) => new Set(prev).add(target.projectId!));
      }
      setLoading(true);
      void getTargetSkillStates(targetId)
        .then((skills) => {
          setAppState((prev) =>
            prev
              ? {
                  ...prev,
                  selectedTargetId: targetId,
                  selectedTargetSkills: skills,
                }
              : prev
          );
          setError(null);
        })
        .catch((err) => setError(errorMessage(err)))
        .finally(() => setLoading(false));
    },
    [
      appState,
      setAppState,
      setError,
      setExpandedProjectIds,
      setLoading,
      setMainView,
      setSelectedProjectId,
      setSelectedTargetId,
    ]
  );

  const handleOpenSkillHub = useCallback(() => {
    setMainView('skill-hub');
    void refreshHub().catch((err) => setError(errorMessage(err)));
  }, [refreshHub, setError, setMainView]);

  const handleToggleSkill = useCallback(
    async (skillKey: string, state: SkillInstallState) => {
      if (!appState || !selectedTargetId) return;

      if (canForceClearInstallation(state)) {
        setForceClearSkillKey(skillKey);
        setForceClearSkillConfirmOpen(true);
        return;
      }

      setPendingSkillKey(skillKey);
      try {
        const next =
          state === 'notInstalled'
            ? await installSkill(selectedTargetId, skillKey)
            : await uninstallSkill(selectedTargetId, skillKey, false);
        applyRemoteState(next);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setPendingSkillKey(null);
      }
    },
    [
      appState,
      applyRemoteState,
      selectedTargetId,
      setError,
      setForceClearSkillConfirmOpen,
      setForceClearSkillKey,
    ]
  );

  const handleConfirmForceClearSkill = useCallback(async () => {
    if (!selectedTargetId || !forceClearSkillKey) return;
    const skillKey = forceClearSkillKey;
    setPendingSkillKey(skillKey);
    try {
      const next = await uninstallSkill(selectedTargetId, skillKey, true);
      setForceClearSkillConfirmOpen(false);
      setForceClearSkillKey(null);
      applyRemoteState(next);
      setError(cleanupWarningsMessage(next));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  }, [
    applyRemoteState,
    forceClearSkillKey,
    selectedTargetId,
    setError,
    setForceClearSkillConfirmOpen,
    setForceClearSkillKey,
  ]);

  const handleCancelForceClearSkill = useCallback(() => {
    setForceClearSkillConfirmOpen(false);
    setForceClearSkillKey(null);
  }, [setForceClearSkillConfirmOpen, setForceClearSkillKey]);

  const handleDeleteMainSkill = useCallback(
    (storageKey: string, displayName: string) => {
      setDeleteSkillStorageKey(storageKey);
      setDeleteSkillDirName(displayName);
    },
    [setDeleteSkillDirName, setDeleteSkillStorageKey]
  );

  const handleConfirmDeleteMainSkill = useCallback(async () => {
    if (!deleteSkillStorageKey || !appState) return;
    setPendingSkillKey(`delete-skill-${deleteSkillStorageKey}`);
    const storageKey = deleteSkillStorageKey;
    setDeleteSkillStorageKey(null);
    setDeleteSkillDirName(null);
    try {
      const next = await deleteMainSkill(storageKey, true);
      applyRemoteState(next);
      syncHubFromAppState(next);
      await refreshHub();
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
      await refresh(selectedTargetId);
    } finally {
      setPendingSkillKey(null);
    }
  }, [
    appState,
    applyRemoteState,
    deleteSkillStorageKey,
    refresh,
    refreshHub,
    selectedTargetId,
    setDeleteSkillDirName,
    setDeleteSkillStorageKey,
    setError,
    syncHubFromAppState,
  ]);

  const handleCancelDeleteMainSkill = useCallback(() => {
    setDeleteSkillDirName(null);
    setDeleteSkillStorageKey(null);
  }, [setDeleteSkillDirName, setDeleteSkillStorageKey]);

  return {
    pendingSkillKey,
    setPendingSkillKey,
    syncDialog,
    closeSyncDialog,
    openManualSyncDialog,
    handleSyncFromTarget,
    handleSetMainSkillsDir,
    handleConfirmSetMainSkillsDir,
    handleAddGlobalTarget,
    handleAddProjectTarget,
    handleAddTargetSuccess,
    handleEditTarget,
    handleConfirmEditTarget,
    handleDeleteTarget,
    handleConfirmDeleteTarget,
    handleCancelDeleteTarget,
    handleSelectTarget,
    handleOpenSkillHub,
    handleToggleSkill,
    handleConfirmForceClearSkill,
    handleCancelForceClearSkill,
    handleDeleteMainSkill,
    handleConfirmDeleteMainSkill,
    handleCancelDeleteMainSkill,
  };
}
