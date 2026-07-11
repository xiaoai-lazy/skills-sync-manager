import { useCallback, type Dispatch, type SetStateAction } from 'react';
import type { AppState, Project } from '../model/types';
import { deleteProject } from '../api/commands';
import { cleanupWarningsMessage } from '../utils/cleanupWarnings';
import { errorMessage } from '../utils/errorMessage';
import { errorCode } from '../utils/ipcError';

export type ProjectFormDialogState = {
  open: boolean;
  mode: 'add' | 'edit';
  project?: Project;
};

export function useProjectActions(args: {
  applyAppStateSuccess: (next: AppState) => void;
  selectedTargetId: string | null;
  selectedProjectId: string | null;
  setSelectedProjectId: (id: string | null) => void;
  setExpandedProjectIds: Dispatch<SetStateAction<Set<string>>>;
  setError: (message: string | null) => void;
  setPendingSkillKey: (key: string | null) => void;
  projectFormDialog: ProjectFormDialogState;
  setProjectFormDialog: Dispatch<SetStateAction<ProjectFormDialogState>>;
  deleteProjectData: Project | null;
  setDeleteProjectData: (project: Project | null) => void;
  setDeleteProjectConfirmOpen: (open: boolean) => void;
  deleteProjectForce: boolean;
  setDeleteProjectForce: (force: boolean) => void;
}) {
  const {
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
  } = args;

  const handleAddProject = useCallback(() => {
    setProjectFormDialog({ open: true, mode: 'add' });
  }, [setProjectFormDialog]);

  const handleEditProject = useCallback(
    (project: Project) => {
      setProjectFormDialog({ open: true, mode: 'edit', project });
    },
    [setProjectFormDialog]
  );

  const handleProjectFormSuccess = useCallback(
    (next: AppState) => {
      setProjectFormDialog((prev) => ({ ...prev, open: false }));
      applyAppStateSuccess(next);
      if (projectFormDialog.mode === 'add') {
        const newest = next.config.projects[next.config.projects.length - 1];
        if (newest) {
          setSelectedProjectId(newest.id);
          setExpandedProjectIds((prev) => new Set(prev).add(newest.id));
        }
      }
    },
    [
      applyAppStateSuccess,
      projectFormDialog.mode,
      setExpandedProjectIds,
      setProjectFormDialog,
      setSelectedProjectId,
    ]
  );

  const handleToggleProject = useCallback(
    (projectId: string) => {
      setExpandedProjectIds((prev) => {
        const next = new Set(prev);
        if (next.has(projectId)) {
          next.delete(projectId);
        } else {
          next.add(projectId);
        }
        return next;
      });
      setSelectedProjectId(projectId);
    },
    [setExpandedProjectIds, setSelectedProjectId]
  );

  const handleDeleteProject = useCallback(
    (project: Project) => {
      setDeleteProjectForce(false);
      setDeleteProjectData(project);
      setDeleteProjectConfirmOpen(true);
    },
    [setDeleteProjectConfirmOpen, setDeleteProjectData, setDeleteProjectForce]
  );

  const handleConfirmDeleteProject = useCallback(async () => {
    if (!deleteProjectData) return;
    const project = deleteProjectData;
    const force = deleteProjectForce;
    setPendingSkillKey(`delete-project-${project.id}`);
    try {
      const next = await deleteProject(project.id, selectedTargetId, force);
      setDeleteProjectConfirmOpen(false);
      setDeleteProjectData(null);
      setDeleteProjectForce(false);
      applyAppStateSuccess(next);
      if (selectedProjectId === project.id) {
        setSelectedProjectId(null);
      }
      setExpandedProjectIds((prev) => {
        const nextSet = new Set(prev);
        nextSet.delete(project.id);
        return nextSet;
      });
      setError(cleanupWarningsMessage(next));
    } catch (err) {
      if (
        !force &&
        errorCode(err) === 'projectHasTargetsWithInstallations'
      ) {
        setDeleteProjectForce(true);
      } else {
        setError(errorMessage(err));
      }
    } finally {
      setPendingSkillKey(null);
    }
  }, [
    applyAppStateSuccess,
    deleteProjectData,
    deleteProjectForce,
    selectedProjectId,
    selectedTargetId,
    setDeleteProjectConfirmOpen,
    setDeleteProjectData,
    setDeleteProjectForce,
    setError,
    setExpandedProjectIds,
    setPendingSkillKey,
    setSelectedProjectId,
  ]);

  const handleCancelDeleteProject = useCallback(() => {
    setDeleteProjectConfirmOpen(false);
    setDeleteProjectData(null);
    setDeleteProjectForce(false);
  }, [setDeleteProjectConfirmOpen, setDeleteProjectData, setDeleteProjectForce]);

  return {
    handleAddProject,
    handleEditProject,
    handleProjectFormSuccess,
    handleToggleProject,
    handleDeleteProject,
    handleConfirmDeleteProject,
    handleCancelDeleteProject,
  };
}
