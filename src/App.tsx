import { useEffect, useRef } from 'react';

import { selectDirectory } from './api/dialog';

import Sidebar from './components/Sidebar';

import SkillHubPage from './components/skill-hub/SkillHubPage';

import TargetDetail from './components/TargetDetail';

import ConfirmDialog from './components/ConfirmDialog';

import PromptDialog from './components/PromptDialog';

import TargetFormDialog from './components/TargetFormDialog';
import AddTargetDialog from './components/AddTargetDialog';
import ProjectFormDialog from './components/ProjectFormDialog';
import WindowControls from './components/WindowControls';
import UpdateDialog from './components/UpdateDialog';
import { checkAppUpdate, installAppUpdate } from './api/updater';
import { errorMessage } from './utils/errorMessage';

import { useAppDialogs } from './hooks/useAppDialogs';

import { useAppState } from './hooks/useAppState';

import { useAppBootstrap } from './hooks/useAppBootstrap';

import { useSkillHub } from './hooks/useSkillHub';

import { useTargetActions } from './hooks/useTargetActions';

import { useProjectActions } from './hooks/useProjectActions';

function App() {

  const {
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
  } = useAppState();

  const {
    promptDialogOpen,
    setPromptDialogOpen,
    promptDialogDefaultValue,
    setPromptDialogDefaultValue,
    targetFormOpen,
    setTargetFormOpen,
    targetFormTarget,
    setTargetFormTarget,
    addTargetDialog,
    setAddTargetDialog,
    projectFormDialog,
    setProjectFormDialog,
    deleteProjectConfirmOpen,
    setDeleteProjectConfirmOpen,
    deleteProjectData,
    setDeleteProjectData,
    deleteTargetConfirmOpen,
    setDeleteTargetConfirmOpen,
    deleteTargetData,
    setDeleteTargetData,
    deleteTargetForce,
    setDeleteTargetForce,
    deleteProjectForce,
    setDeleteProjectForce,
    forceClearSkillKey,
    setForceClearSkillKey,
    forceClearSkillConfirmOpen,
    setForceClearSkillConfirmOpen,
    deleteSkillDirName,
    setDeleteSkillDirName,
    deleteSkillStorageKey,
    setDeleteSkillStorageKey,
    updateDismissedRef,
    updateCheckStartedRef,
    updateDialogOpen,
    setUpdateDialogOpen,
    updateInfo,
    setUpdateInfo,
    updateInstalling,
    setUpdateInstalling,
    updateError,
    setUpdateError,
  } = useAppDialogs();

  const setErrorRef = useRef<(message: string | null) => void>(() => {});

  const {
    hubState,
    discoverSkillsList,
    setDiscoverSkillsList,
    pendingUpdates,
    setPendingUpdates,
    syncFromAppState: syncHubFromAppState,
    refreshHub,
    runBackgroundDiscover,
    runBackgroundCheckUpdates,
  } = useSkillHub({
    appState,
    setAppState,
    setError: (message) => setErrorRef.current(message),
  });

  const {
    loading,
    setLoading,
    error,
    setError,
    migrationToast,
    setMigrationToast,
    migrationToastIsError,
    applyAppStateSuccess,
    refresh,
  } = useAppBootstrap({
    appState,
    selectedTargetId,
    applyRemoteState,
    syncFromAppState: syncHubFromAppState,
    runBackgroundDiscover,
    runBackgroundCheckUpdates,
  });

  setErrorRef.current = setError;

  useEffect(() => {

    if (!appState || updateDismissedRef.current || updateCheckStartedRef.current) return;

    updateCheckStartedRef.current = true;

    void checkAppUpdate()

      .then((info) => {

        if (info) {

          setUpdateInfo(info);

          setUpdateDialogOpen(true);

        }

      })

      .catch(() => {

        /* ignore update check failures at startup */

      });

  }, [appState]);

  const handleDeferUpdate = () => {

    updateDismissedRef.current = true;

    setUpdateDialogOpen(false);

    setUpdateError(null);

  };

  const handleInstallUpdate = async () => {

    setUpdateInstalling(true);

    setUpdateError(null);

    try {

      await installAppUpdate();

    } catch (err) {

      setUpdateError(errorMessage(err));

      setUpdateInstalling(false);

    }

  };

  const {
    pendingSkillKey,
    setPendingSkillKey,
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
  } = useTargetActions({
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
  });

  const {
    handleAddProject,
    handleEditProject,
    handleProjectFormSuccess,
    handleToggleProject,
    handleDeleteProject,
    handleConfirmDeleteProject,
    handleCancelDeleteProject,
  } = useProjectActions({
    applyAppStateSuccess,
    selectedTargetId,
    selectedProjectId,
    setSelectedProjectId,
    setExpandedProjectIds,
    setError,
    setPendingSkillKey,
    projectFormDialog,
    setProjectFormDialog,
    deleteProjectData,
    setDeleteProjectData,
    setDeleteProjectConfirmOpen,
    deleteProjectForce,
    setDeleteProjectForce,
  });

  const mainSkillsDir = appState?.config.settings.mainSkillsDir ?? null;

  const selectedTarget =

    appState?.config.targets.find((t) => t.id === selectedTargetId) ?? null;

  const deleteLinkCount = deleteSkillStorageKey

    ? appState?.config.installations.filter(

        (i) => i.skillStorageKey === deleteSkillStorageKey

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

        projects={appState?.config.projects ?? []}

        selectedTargetId={selectedTargetId}

        selectedProjectId={selectedProjectId}

        expandedProjectIds={expandedProjectIds}

        mainView={mainView}

        onOpenSkillHub={handleOpenSkillHub}

        onSelectTarget={handleSelectTarget}

        onToggleProject={handleToggleProject}

        onAddGlobalTarget={handleAddGlobalTarget}

        onAddProject={handleAddProject}

        onAddProjectTarget={handleAddProjectTarget}

        onEditTarget={handleEditTarget}

        onEditProject={handleEditProject}

        onDeleteTarget={handleDeleteTarget}

        onDeleteProject={handleDeleteProject}

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

              skillHubEndpoints={appState.config.skillHubEndpoints}

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

            skillRecords={appState?.config.skillRecords ?? {}}

            endpoints={appState?.config.skillHubEndpoints ?? []}

            repos={appState?.config.skillRepos ?? []}

            pendingSkillKey={pendingSkillKey}

            onToggleSkill={handleToggleSkill}

          />

        )}

        <ConfirmDialog

          open={!!deleteSkillStorageKey}

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

        <AddTargetDialog

          open={addTargetDialog.open}

          onClose={() => setAddTargetDialog((prev) => ({ ...prev, open: false }))}

          scope={addTargetDialog.scope}

          projectId={addTargetDialog.projectId}

          projectName={addTargetDialog.projectName}

          existingTargets={appState?.config.targets ?? []}

          selectedTargetId={selectedTargetId}

          onSuccess={handleAddTargetSuccess}

        />

        <ProjectFormDialog

          open={projectFormDialog.open}

          onClose={() => setProjectFormDialog((prev) => ({ ...prev, open: false }))}

          mode={projectFormDialog.mode}

          project={projectFormDialog.project}

          selectedTargetId={selectedTargetId}

          onSuccess={handleProjectFormSuccess}

        />

        <TargetFormDialog

          open={targetFormOpen}

          title="编辑目标"

          initialName={targetFormTarget?.name}

          initialSkillsDir={targetFormTarget?.skillsDir}

          skillsDirReadOnly

          confirmLabel="保存"

          onConfirm={(name) => {

            if (targetFormTarget) {

              void handleConfirmEditTarget(targetFormTarget.id, name);

            }

          }}

          onCancel={() => {

            setTargetFormOpen(false);

            setTargetFormTarget(null);

          }}

        />

        <UpdateDialog

          open={updateDialogOpen}

          update={updateInfo}

          installing={updateInstalling}

          error={updateError}

          onDefer={handleDeferUpdate}

          onInstall={() => {

            void handleInstallUpdate();

          }}

        />

        <ConfirmDialog

          open={deleteProjectConfirmOpen}

          title={deleteProjectForce ? '强制删除项目' : '删除项目'}

          message={

            deleteProjectForce

              ? `项目「${deleteProjectData?.name}」下仍有安装记录。将尽力移除正常链接并清除全部安装记录（异常路径需手动清理），然后删除项目。是否继续？`

              : `确定删除项目「${deleteProjectData?.name}」吗？`

          }

          confirmLabel={deleteProjectForce ? '强制删除' : '删除'}

          cancelLabel="取消"

          danger

          onConfirm={handleConfirmDeleteProject}

          onCancel={handleCancelDeleteProject}

        />

        <ConfirmDialog

          open={deleteTargetConfirmOpen}

          title={deleteTargetForce ? '强制删除目标' : '删除目标'}

          message={

            deleteTargetForce

              ? `目标「${deleteTargetData?.name}」仍有安装记录。将尽力移除正常链接并清除全部安装记录（异常路径需手动清理），然后删除目标。是否继续？`

              : `确定删除目标「${deleteTargetData?.name}」吗？`

          }

          confirmLabel={deleteTargetForce ? '强制删除' : '删除'}

          cancelLabel="取消"

          danger

          onConfirm={handleConfirmDeleteTarget}

          onCancel={handleCancelDeleteTarget}

        />

        <ConfirmDialog

          open={forceClearSkillConfirmOpen}

          title="强制清除安装记录"

          message="将清除该 Skill 的安装记录；若链接正常会一并移除。链接异常或非本程序创建的内容会保留在目标目录，需手动清理。是否继续？"

          confirmLabel="强制清除"

          cancelLabel="取消"

          danger

          onConfirm={handleConfirmForceClearSkill}

          onCancel={handleCancelForceClearSkill}

        />

      </main>

    </div>

    {migrationToast && (

      <div

        className={`migration-toast${migrationToastIsError ? ' migration-toast--error' : ''}`}

        role="status"

      >

        <span>{migrationToast}</span>

        <button

          className="close-button"

          onClick={() => setMigrationToast(null)}

          aria-label="关闭提示"

        >

          ×

        </button>

      </div>

    )}

    </div>

  );

}

export default App;

