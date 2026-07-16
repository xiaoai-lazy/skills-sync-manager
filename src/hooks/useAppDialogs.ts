import { useState } from 'react';
import type { Project, Target, TargetScope } from '../model/types';

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

  const [deleteProjectForce, setDeleteProjectForce] = useState(false);

  const [forceClearSkillKey, setForceClearSkillKey] = useState<string | null>(null);
  const [forceClearSkillConfirmOpen, setForceClearSkillConfirmOpen] = useState(false);

  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);
  const [deleteSkillStorageKey, setDeleteSkillStorageKey] = useState<string | null>(null);

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
  };
}
