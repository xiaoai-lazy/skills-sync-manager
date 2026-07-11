import { invoke } from '@tauri-apps/api/core';
import type { AgentPreset, AppState, TargetScope } from '../model/types';

export async function getAppState(selectedTargetId?: string | null): Promise<AppState> {
  return invoke<AppState>('get_app_state', { selectedTargetId: selectedTargetId ?? null });
}

export async function setMainSkillsDir(path: string): Promise<AppState> {
  return invoke<AppState>('set_main_skills_dir', { path });
}

export async function updateTarget(targetId: string, name: string): Promise<AppState> {
  return invoke<AppState>('update_target', { targetId, name });
}

export async function deleteTarget(targetId: string, cleanupRecordedLinks: boolean): Promise<AppState> {
  return invoke<AppState>('delete_target', { targetId, cleanupRecordedLinks });
}

export async function installSkill(targetId: string, skillIdentifier: string): Promise<AppState> {
  return invoke<AppState>('install_skill', { targetId, skillIdentifier });
}

export async function uninstallSkill(
  targetId: string,
  skillIdentifier: string,
  force = false,
): Promise<AppState> {
  return invoke<AppState>('uninstall_skill', {
    targetId,
    skillIdentifier,
    force,
  });
}

export async function deleteMainSkill(
  skillIdentifier: string,
  confirmed: boolean,
): Promise<AppState> {
  return invoke<AppState>('delete_main_skill', {
    skillIdentifier,
    confirmed,
  });
}

export async function listAgentPresets(
  scope: TargetScope,
  projectId?: string | null,
): Promise<AgentPreset[]> {
  return invoke<AgentPreset[]>('list_agent_presets', {
    scope,
    projectId: projectId ?? null,
  });
}

export async function addAgentTarget(
  scope: TargetScope,
  agentId: string,
  projectId?: string | null,
  selectedTargetId?: string | null,
): Promise<AppState> {
  return invoke<AppState>('add_agent_target', {
    scope,
    agentId,
    projectId: projectId ?? null,
    selectedTargetId: selectedTargetId ?? null,
  });
}

export async function addCustomTarget(
  scope: TargetScope,
  name: string,
  skillsDir: string,
  projectId?: string | null,
  selectedTargetId?: string | null,
): Promise<AppState> {
  return invoke<AppState>('add_custom_target', {
    scope,
    name,
    skillsDir,
    projectId: projectId ?? null,
    selectedTargetId: selectedTargetId ?? null,
  });
}

export async function addProject(
  name: string,
  rootPath: string,
  selectedTargetId?: string | null,
): Promise<AppState> {
  return invoke<AppState>('add_project', {
    name,
    rootPath,
    selectedTargetId: selectedTargetId ?? null,
  });
}

export async function updateProject(
  projectId: string,
  name: string,
  selectedTargetId?: string | null,
): Promise<AppState> {
  return invoke<AppState>('update_project', {
    projectId,
    name,
    selectedTargetId: selectedTargetId ?? null,
  });
}

export async function deleteProject(
  projectId: string,
  selectedTargetId?: string | null,
  cleanupRecordedLinks = false,
): Promise<AppState> {
  return invoke<AppState>('delete_project', {
    projectId,
    selectedTargetId: selectedTargetId ?? null,
    cleanupRecordedLinks,
  });
}
