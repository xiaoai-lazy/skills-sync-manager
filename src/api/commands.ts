import { invoke } from '@tauri-apps/api/core';
import type { AppState } from '../model/types';

export async function getAppState(selectedTargetId?: string | null): Promise<AppState> {
  return invoke<AppState>('get_app_state', { selectedTargetId: selectedTargetId ?? null });
}

export async function setMainSkillsDir(path: string): Promise<AppState> {
  return invoke<AppState>('set_main_skills_dir', { path });
}

export async function addTarget(name: string, skillsDir: string): Promise<AppState> {
  return invoke<AppState>('add_target', { name, skillsDir });
}

export async function updateTarget(targetId: string, name: string, skillsDir: string): Promise<AppState> {
  return invoke<AppState>('update_target', { targetId, name, skillsDir });
}

export async function deleteTarget(targetId: string, cleanupRecordedLinks: boolean): Promise<AppState> {
  return invoke<AppState>('delete_target', { targetId, cleanupRecordedLinks });
}

export async function installSkill(targetId: string, skillDirName: string): Promise<AppState> {
  return invoke<AppState>('install_skill', { targetId, skillDirName });
}

export async function uninstallSkill(targetId: string, skillDirName: string): Promise<AppState> {
  return invoke<AppState>('uninstall_skill', { targetId, skillDirName });
}

export async function deleteMainSkill(skillDirName: string, confirmed: boolean): Promise<AppState> {
  return invoke<AppState>('delete_main_skill', { skillDirName, confirmed });
}
