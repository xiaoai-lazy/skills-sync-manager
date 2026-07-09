import { useRef, useState } from 'react';
import type { Project, Target, TargetScope } from '../model/types';
import type { UpdateInfo } from '../api/updater';

export type AddTargetDialogState = {
  open: boolean;
  scope: TargetScope;
  projectId?: string;
  projectName?: string;
};

export type ProjectFormDialogState = {
  open: boolean;
  mode: 'add' | 'edit';
  project?: Project;
};

export function useAppDialogs() {
  const [promptDialogOpen, setPromptDialogOpen] = useState(false);
  const [promptDialogDefaultValue, setPromptDialogDefaultValue] = useState('');

  const [targetFormOpen, setTargetFormOpen] = useState(false);
  const [targetFormTarget, setTargetFormTarget] = useState<Target | null>(null);

  const [addTargetDialog, setAddTargetDialog] = useState<AddTargetDialogState>({
    open: false,
    scope: 'global',
  });

  const [projectFormDialog, setProjectFormDialog] = useState<ProjectFormDialogState>({
    open: false,
    mode: 'add',
  });

  const [deleteProjectConfirmOpen, setDeleteProjectConfirmOpen] = useState(false);
  const [deleteProjectData, setDeleteProjectData] = useState<Project | null>(null);

  const [deleteTargetConfirmOpen, setDeleteTargetConfirmOpen] = useState(false);
  const [deleteTargetData, setDeleteTargetData] = useState<Target | null>(null);
  const [deleteTargetForce, setDeleteTargetForce] = useState(false);

  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);
  const [deleteSkillStorageKey, setDeleteSkillStorageKey] = useState<string | null>(null);

  const updateDismissedRef = useRef(false);
  const updateCheckStartedRef = useRef(false);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateInstalling, setUpdateInstalling] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);

  return {
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
  };
}
