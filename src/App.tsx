import React, { useState, useEffect, useCallback } from 'react';
import type { AppState, Target, SkillWithTargetState, SkillInstallState } from './model/types';
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
import Sidebar from './components/Sidebar';
import MainLibraryPage from './components/MainLibraryPage';
import TargetDetail from './components/TargetDetail';
import ConfirmDialog from './components/ConfirmDialog';
import PromptDialog from './components/PromptDialog';
import TargetFormDialog from './components/TargetFormDialog';

function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return '操作失败，请查看日志或重试。';
}

type MainView = 'main-library' | 'target';

function App() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
  const [mainView, setMainView] = useState<MainView>('main-library');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);
  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);

  const [promptDialogOpen, setPromptDialogOpen] = useState(false);
  const [promptDialogDefaultValue, setPromptDialogDefaultValue] = useState('');

  const [targetFormOpen, setTargetFormOpen] = useState(false);
  const [targetFormTarget, setTargetFormTarget] = useState<Target | null>(null);

  const [deleteTargetConfirmOpen, setDeleteTargetConfirmOpen] = useState(false);
  const [deleteTargetData, setDeleteTargetData] = useState<Target | null>(null);
  const [deleteTargetForce, setDeleteTargetForce] = useState(false);

  const refresh = useCallback(
    async (nextSelectedTargetId: string | null = selectedTargetId): Promise<void> => {
      setLoading(true);
      try {
        const next = await getAppState(nextSelectedTargetId);
        setAppState(next);
        setSelectedTargetId(next.selectedTargetId);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setLoading(false);
      }
    },
    [selectedTargetId]
  );

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
      setMainView('target');
      await refresh(next.selectedTargetId);
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
    refresh(targetId);
  };

  const handleManageSkills = () => {
    setMainView('main-library');
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
  const validSkills = appState?.skills.filter((s) => s.valid) ?? [];
  const invalidSkills = appState?.skills.filter((s) => !s.valid) ?? [];
  const selectedTarget =
    appState?.config.targets.find((t) => t.id === selectedTargetId) ?? null;

  const deleteLinkCount = deleteSkillDirName
    ? appState?.config.installations.filter(
        (i) => i.skillDirName === deleteSkillDirName
      ).length ?? 0
    : 0;

  const deleteMessage = deleteSkillDirName
    ? deleteLinkCount > 0
      ? `Skill '${deleteSkillDirName}' will be permanently deleted. ${deleteLinkCount} recorded target link(s) will be removed first. This action cannot be undone.`
      : `Skill '${deleteSkillDirName}' will be permanently deleted. This action cannot be undone.`
    : '';

  return (
    <div className="app-shell">
      <Sidebar
        mainSkillsDir={mainSkillsDir}
        validSkillCount={validSkills.length}
        invalidSkillCount={invalidSkills.length}
        targets={appState?.config.targets ?? []}
        selectedTargetId={selectedTargetId}
        onSelectTarget={handleSelectTarget}
        onAddTarget={handleAddTarget}
        onEditTarget={handleEditTarget}
        onDeleteTarget={handleDeleteTarget}
        onSetMainSkillsDir={handleSetMainSkillsDir}
        onManageSkills={handleManageSkills}
      />
      <main className="main-panel">
        {loading && <div className="loading-overlay">Loading…</div>}
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
        {mainView === 'main-library' ? (
          <MainLibraryPage
            skills={appState?.skills ?? []}
            validSkillCount={validSkills.length}
            invalidSkillCount={invalidSkills.length}
            onDeleteMainSkill={handleDeleteMainSkill}
          />
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
          title="Confirm Deletion"
          message={deleteMessage}
          confirmLabel="Delete"
          cancelLabel="Cancel"
          danger
          onConfirm={handleConfirmDeleteMainSkill}
          onCancel={handleCancelDeleteMainSkill}
        />
        <PromptDialog
          open={promptDialogOpen}
          title="Set Main Skills Directory"
          label="Main skills directory path"
          defaultValue={promptDialogDefaultValue}
          confirmLabel="Save"
          onConfirm={handleConfirmSetMainSkillsDir}
          onCancel={() => setPromptDialogOpen(false)}
        />
        <TargetFormDialog
          open={targetFormOpen}
          title={targetFormTarget ? 'Edit Target' : 'Add Target'}
          initialName={targetFormTarget?.name}
          initialSkillsDir={targetFormTarget?.skillsDir}
          confirmLabel={targetFormTarget ? 'Save' : 'Add'}
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
          title={deleteTargetForce ? 'Force Delete Target' : 'Delete Target'}
          message={
            deleteTargetForce
              ? `Target "${deleteTargetData?.name}" has recorded installations. Remove links and delete target?`
              : `Delete target "${deleteTargetData?.name}"?`
          }
          confirmLabel={deleteTargetForce ? 'Force Delete' : 'Delete'}
          cancelLabel="Cancel"
          danger
          onConfirm={handleConfirmDeleteTarget}
          onCancel={handleCancelDeleteTarget}
        />
      </main>
    </div>
  );
}

export default App;
