import React, { useState, useEffect, useCallback, useRef } from 'react';

import type {

  AppState,

  Target,

  SkillInstallState,

  SkillHubLocalState,

  DiscoverableSkill,

  SkillUpdateInfo,

} from './model/types';

import {

  getAppState,

  setMainSkillsDir,

  addTarget,

  updateTarget,

  deleteTarget,

  installSkill,

  uninstallSkill,

  deleteMainSkill,

} from './api/commands';

import {

  scanMainLibrary,

  discoverSkills,

  checkSkillUpdates,

  getTargetSkillStates,

} from './api/skillHub';

import { selectDirectory } from './api/dialog';

import Sidebar from './components/Sidebar';

import SkillHubPage from './components/skill-hub/SkillHubPage';

import TargetDetail from './components/TargetDetail';

import ConfirmDialog from './components/ConfirmDialog';

import PromptDialog from './components/PromptDialog';

import TargetFormDialog from './components/TargetFormDialog';
import WindowControls from './components/WindowControls';
import { errorMessage } from './utils/errorMessage';

type MainView = 'skill-hub' | 'target';



const emptyHubState: SkillHubLocalState = {

  skills: [],

  validCount: 0,

  invalidCount: 0,

  pendingUpdateCount: 0,

  lastScanAt: '',

  skillRecords: {},

};



function buildHubStateFromAppState(state: AppState): SkillHubLocalState {

  const validCount = state.skills.filter((s) => s.valid).length;

  const invalidCount = state.skills.length - validCount;

  return {

    skills: state.skills,

    validCount,

    invalidCount,

    pendingUpdateCount: state.config.skillUpdateCache?.updates?.length ?? 0,

    lastScanAt: new Date().toISOString(),

    skillRecords: state.config.skillRecords ?? {},

  };

}



function App() {

  const [appState, setAppState] = useState<AppState | null>(null);

  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);

  const [mainView, setMainView] = useState<MainView>('skill-hub');

  const [loading, setLoading] = useState(true);

  const [error, setError] = useState<string | null>(null);

  const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);

  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);



  const [hubState, setHubState] = useState<SkillHubLocalState>(emptyHubState);

  const [discoverSkillsList, setDiscoverSkillsList] = useState<DiscoverableSkill[]>([]);

  const [pendingUpdates, setPendingUpdates] = useState<SkillUpdateInfo[]>([]);



  const discoverInFlight = useRef(false);

  const checkInFlight = useRef(false);

  const startupBackgroundDone = useRef(false);



  const [promptDialogOpen, setPromptDialogOpen] = useState(false);

  const [promptDialogDefaultValue, setPromptDialogDefaultValue] = useState('');



  const [targetFormOpen, setTargetFormOpen] = useState(false);

  const [targetFormTarget, setTargetFormTarget] = useState<Target | null>(null);



  const [deleteTargetConfirmOpen, setDeleteTargetConfirmOpen] = useState(false);

  const [deleteTargetData, setDeleteTargetData] = useState<Target | null>(null);

  const [deleteTargetForce, setDeleteTargetForce] = useState(false);



  const syncHubFromAppState = useCallback((state: AppState) => {

    setHubState(buildHubStateFromAppState(state));

    setDiscoverSkillsList(state.config.skillDiscoverCache?.skills ?? []);

    setPendingUpdates(state.config.skillUpdateCache?.updates ?? []);

  }, []);



  const applyHubState = useCallback((next: SkillHubLocalState) => {

    setHubState(next);

    setAppState((prev) => {

      if (!prev) return prev;

      return {

        ...prev,

        config: {

          ...prev.config,

          skillRecords: next.skillRecords,

        },

      };

    });

  }, []);



  const refreshHub = useCallback(async (): Promise<void> => {

    const next = await scanMainLibrary();

    applyHubState(next);

  }, [applyHubState]);



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

      setError(errorMessage(err));

    } finally {

      discoverInFlight.current = false;

    }

  }, []);



  const runBackgroundCheckUpdates = useCallback(async (): Promise<void> => {

    if (checkInFlight.current) return;

    checkInFlight.current = true;

    try {

      const updates = await checkSkillUpdates();

      setPendingUpdates(updates);

      const next = await scanMainLibrary();

      applyHubState(next);

    } catch (err) {

      setError(errorMessage(err));

    } finally {

      checkInFlight.current = false;

    }

  }, [applyHubState]);



  const refresh = useCallback(

    async (nextSelectedTargetId: string | null = selectedTargetId): Promise<void> => {

      setLoading(true);

      try {

        const next = await getAppState(nextSelectedTargetId);

        setAppState(next);

        setSelectedTargetId(next.selectedTargetId);

        syncHubFromAppState(next);

        setError(null);

      } catch (err) {

        setError(errorMessage(err));

      } finally {

        setLoading(false);

      }

    },

    [selectedTargetId, syncHubFromAppState]

  );



  useEffect(() => {

    refresh();

    // eslint-disable-next-line react-hooks/exhaustive-deps

  }, []);



  useEffect(() => {

    if (!appState || startupBackgroundDone.current) return;

    startupBackgroundDone.current = true;

    void Promise.all([runBackgroundDiscover(), runBackgroundCheckUpdates()]);

  }, [appState, runBackgroundDiscover, runBackgroundCheckUpdates]);



  const handleSetMainSkillsDir = () => {

    if (!appState) return;

    setPromptDialogDefaultValue(appState.config.settings.mainSkillsDir ?? '');

    setPromptDialogOpen(true);

  };



  const handleConfirmSetMainSkillsDir = async (path: string) => {

    setPromptDialogOpen(false);

    setPendingSkillKey('mainDir');

    try {

      const next = await setMainSkillsDir(path);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      syncHubFromAppState(next);

      await refreshHub();

      setError(null);

    } catch (err) {

      setError(errorMessage(err));

    } finally {

      setPendingSkillKey(null);

    }

  };



  const handleAddTarget = () => {

    setTargetFormTarget(null);

    setTargetFormOpen(true);

  };



  const handleConfirmAddTarget = async (name: string, skillsDir: string) => {

    setTargetFormOpen(false);

    setPendingSkillKey('addTarget');

    try {

      const next = await addTarget(name, skillsDir);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      syncHubFromAppState(next);

      setMainView('target');

      const skills = await getTargetSkillStates(next.selectedTargetId!);

      setAppState((prev) =>

        prev ? { ...prev, selectedTargetSkills: skills } : prev

      );

      setError(null);

    } catch (err) {

      setError(errorMessage(err));

    } finally {

      setPendingSkillKey(null);

    }

  };



  const handleEditTarget = (target: Target) => {

    setTargetFormTarget(target);

    setTargetFormOpen(true);

  };



  const handleConfirmEditTarget = async (targetId: string, name: string, skillsDir: string) => {

    setTargetFormOpen(false);

    setPendingSkillKey(`edit-${targetId}`);

    try {

      const next = await updateTarget(targetId, name, skillsDir);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      setError(null);

    } catch (err) {

      setError(errorMessage(err));

    } finally {

      setPendingSkillKey(null);

    }

  };



  const handleDeleteTarget = (target: Target) => {

    setDeleteTargetData(target);

    setDeleteTargetForce(false);

    setDeleteTargetConfirmOpen(true);

  };



  const handleConfirmDeleteTarget = async () => {

    if (!deleteTargetData) return;

    const target = deleteTargetData;

    const force = deleteTargetForce;

    setPendingSkillKey(`delete-${target.id}`);

    try {

      const next = await deleteTarget(target.id, force);

      setDeleteTargetConfirmOpen(false);

      setDeleteTargetData(null);

      setDeleteTargetForce(false);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      setError(null);

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

  };



  const handleCancelDeleteTarget = () => {

    setDeleteTargetConfirmOpen(false);

    setDeleteTargetData(null);

    setDeleteTargetForce(false);

  };



  const handleSelectTarget = (targetId: string) => {

    setMainView('target');

    setSelectedTargetId(targetId);

    setLoading(true);

    void getTargetSkillStates(targetId)

      .then((skills) => {

        setAppState((prev) =>

          prev ? { ...prev, selectedTargetId: targetId, selectedTargetSkills: skills } : prev

        );

        setError(null);

      })

      .catch((err) => setError(errorMessage(err)))

      .finally(() => setLoading(false));

  };



  const handleOpenSkillHub = () => {

    setMainView('skill-hub');

    void refreshHub().catch((err) => setError(errorMessage(err)));

  };



  const handleToggleSkill = async (skillDirName: string, state: SkillInstallState) => {

    if (!appState || !selectedTargetId) return;

    setPendingSkillKey(skillDirName);

    try {

      const next =

        state === 'notInstalled'

          ? await installSkill(selectedTargetId, skillDirName)

          : await uninstallSkill(selectedTargetId, skillDirName);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      setError(null);

    } catch (err) {

      setError(errorMessage(err));

    } finally {

      setPendingSkillKey(null);

    }

  };



  const handleDeleteMainSkill = (skillDirName: string) => {

    setDeleteSkillDirName(skillDirName);

  };



  const handleConfirmDeleteMainSkill = async () => {

    if (!deleteSkillDirName || !appState) return;

    setPendingSkillKey(`delete-skill-${deleteSkillDirName}`);

    setDeleteSkillDirName(null);

    try {

      const next = await deleteMainSkill(deleteSkillDirName, true);

      setAppState(next);

      setSelectedTargetId(next.selectedTargetId);

      syncHubFromAppState(next);

      await refreshHub();

      setError(null);

    } catch (err) {

      setError(errorMessage(err));

      await refresh(selectedTargetId);

    } finally {

      setPendingSkillKey(null);

    }

  };



  const handleCancelDeleteMainSkill = () => {

    setDeleteSkillDirName(null);

  };



  const mainSkillsDir = appState?.config.settings.mainSkillsDir ?? null;

  const selectedTarget =

    appState?.config.targets.find((t) => t.id === selectedTargetId) ?? null;



  const deleteLinkCount = deleteSkillDirName

    ? appState?.config.installations.filter(

        (i) => i.skillDirName === deleteSkillDirName

      ).length ?? 0

    : 0;



  const deleteMessage = deleteSkillDirName

    ? deleteLinkCount > 0

      ? `Skill「${deleteSkillDirName}」将被永久删除。将先移除 ${deleteLinkCount} 条目标链接记录。此操作不可撤销。`

      : `Skill「${deleteSkillDirName}」将被永久删除。此操作不可撤销。`

    : '';



  return (

    <div className="app-frame">

      <header className="app-chrome" data-tauri-drag-region>

        <WindowControls />

      </header>

      <div className="app-shell">

      <Sidebar

        targets={appState?.config.targets ?? []}

        selectedTargetId={selectedTargetId}

        mainView={mainView}

        onOpenSkillHub={handleOpenSkillHub}

        onSelectTarget={handleSelectTarget}

        onAddTarget={handleAddTarget}

        onEditTarget={handleEditTarget}

        onDeleteTarget={handleDeleteTarget}

      />

      <main className="main-panel">

        {loading && <div className="loading-overlay">加载中…</div>}

        {error && (

          <div className="error-banner">

            {error}

            <button

              className="close-button"

              onClick={() => setError(null)}

              aria-label="Dismiss error"

            >

              ×

            </button>

          </div>

        )}

        {mainView === 'skill-hub' ? (

          appState ? (

            <SkillHubPage

              mainSkillsDir={mainSkillsDir}

              hubState={hubState}

              discoverSkills={discoverSkillsList}

              pendingUpdates={pendingUpdates}

              skillRecords={appState.config.skillRecords}

              onHubStateChange={applyHubState}

              onDiscoverSkillsChange={setDiscoverSkillsList}

              onPendingUpdatesChange={setPendingUpdates}

              onDeleteMainSkill={handleDeleteMainSkill}

              onSetMainSkillsDir={handleSetMainSkillsDir}

              onRefreshHub={refreshHub}

              onError={(err) => setError(errorMessage(err))}

            />

          ) : null

        ) : (

          <TargetDetail

            target={selectedTarget}

            skills={appState?.selectedTargetSkills ?? []}

            pendingSkillKey={pendingSkillKey}

            onToggleSkill={handleToggleSkill}

          />

        )}

        <ConfirmDialog

          open={!!deleteSkillDirName}

          title="确认删除"

          message={deleteMessage}

          confirmLabel="删除"

          cancelLabel="取消"

          danger

          onConfirm={handleConfirmDeleteMainSkill}

          onCancel={handleCancelDeleteMainSkill}

        />

        <PromptDialog

          open={promptDialogOpen}

          title="设置主库目录"

          label="主库目录路径"

          defaultValue={promptDialogDefaultValue}

          confirmLabel="保存"

          pickDirectoryLabel="选择目录"

          onPickDirectory={selectDirectory}

          onConfirm={handleConfirmSetMainSkillsDir}

          onCancel={() => setPromptDialogOpen(false)}

        />

        <TargetFormDialog

          open={targetFormOpen}

          title={targetFormTarget ? '编辑目标' : '添加目标'}

          initialName={targetFormTarget?.name}

          initialSkillsDir={targetFormTarget?.skillsDir}

          confirmLabel={targetFormTarget ? '保存' : '添加'}

          pickDirectoryLabel="选择目录"

          onPickDirectory={selectDirectory}

          onConfirm={

            targetFormTarget

              ? (name, skillsDir) => handleConfirmEditTarget(targetFormTarget.id, name, skillsDir)

              : handleConfirmAddTarget

          }

          onCancel={() => {

            setTargetFormOpen(false);

            setTargetFormTarget(null);

          }}

        />

        <ConfirmDialog

          open={deleteTargetConfirmOpen}

          title={deleteTargetForce ? '强制删除目标' : '删除目标'}

          message={

            deleteTargetForce

              ? `目标「${deleteTargetData?.name}」仍有安装记录。是否移除链接并删除目标？`

              : `确定删除目标「${deleteTargetData?.name}」吗？`

          }

          confirmLabel={deleteTargetForce ? '强制删除' : '删除'}

          cancelLabel="取消"

          danger

          onConfirm={handleConfirmDeleteTarget}

          onCancel={handleCancelDeleteTarget}

        />

      </main>

    </div>

    </div>

  );

}



export default App;

