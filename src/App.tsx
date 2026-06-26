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
import TargetDetail from './components/TargetDetail';
import ConfirmDialog from './components/ConfirmDialog';

function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return '操作失败，请查看日志或重试。';
}

function App() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);
  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);

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

  const handleSetMainSkillsDir = async () => {
    if (!appState) return;
    const path = window.prompt(
      'Enter main skills directory path:',
      appState.config.settings.mainSkillsDir ?? ''
    );
    if (path === null) return;
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

  const handleAddTarget = async () => {
    const name = window.prompt('Enter target name:');
    if (name === null) return;
    const skillsDir = window.prompt('Enter target skills directory path:');
    if (skillsDir === null) return;
    if (!name.trim() || !skillsDir.trim()) return;
    setPendingSkillKey('addTarget');
    try {
      const next = await addTarget(name.trim(), skillsDir.trim());
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleEditTarget = async (target: Target) => {
    const name = window.prompt('Enter new target name:', target.name);
    if (name === null) return;
    const skillsDir = window.prompt('Enter new target skills directory path:', target.skillsDir);
    if (skillsDir === null) return;
    if (!name.trim() || !skillsDir.trim()) return;
    setPendingSkillKey(`edit-${target.id}`);
    try {
      const next = await updateTarget(target.id, name.trim(), skillsDir.trim());
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleDeleteTarget = async (target: Target) => {
    if (!window.confirm(`Delete target "${target.name}"?`)) return;
    setPendingSkillKey(`delete-${target.id}`);
    try {
      const next = await deleteTarget(target.id, false);
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      const msg = errorMessage(err);
      if (
        window.confirm(
          'Target has recorded installations. Remove links and delete target?'
        )
      ) {
        try {
          const next = await deleteTarget(target.id, true);
          setAppState(next);
          setSelectedTargetId(next.selectedTargetId);
          setError(null);
        } catch (err2) {
          setError(errorMessage(err2));
        }
      } else {
        setError(msg);
      }
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleSelectTarget = (targetId: string) => {
    setSelectedTargetId(targetId);
    refresh(targetId);
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
        <TargetDetail
          target={selectedTarget}
          skills={appState?.selectedTargetSkills ?? []}
          pendingSkillKey={pendingSkillKey}
          onToggleSkill={handleToggleSkill}
          onDeleteMainSkill={handleDeleteMainSkill}
        />
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
      </main>
    </div>
  );
}

export default App;
