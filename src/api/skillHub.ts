import { invoke } from '@tauri-apps/api/core';
import type {
  DiscoverableSkill,
  SkillHubLocalState,
  SkillRepo,
  SkillRepoChangeResult,
  SkillUpdateInfo,
  SkillWithTargetState,
  SmartPastePreview,
  UpdateAllSkillsResult,
} from '../model/types';

export async function scanMainLibrary(): Promise<SkillHubLocalState> {
  return invoke<SkillHubLocalState>('scan_main_library');
}

export async function discoverSkills(): Promise<DiscoverableSkill[]> {
  return invoke<DiscoverableSkill[]>('discover_skills');
}

export async function checkSkillUpdates(): Promise<SkillUpdateInfo[]> {
  return invoke<SkillUpdateInfo[]>('check_skill_updates');
}

export async function updateSkill(dirName: string): Promise<void> {
  return invoke<void>('update_skill', { dirName });
}

export async function updateAllSkills(): Promise<UpdateAllSkillsResult> {
  return invoke<UpdateAllSkillsResult>('update_all_skills');
}

export async function parseSmartPaste(input: string): Promise<SmartPastePreview> {
  return invoke<SmartPastePreview>('parse_smart_paste', { input });
}

export async function installHubSkill(
  discoverable: DiscoverableSkill,
): Promise<SkillHubLocalState> {
  return invoke<SkillHubLocalState>('install_hub_skill', { discoverable });
}

export async function getSkillRepos(): Promise<SkillRepo[]> {
  return invoke<SkillRepo[]>('get_skill_repos');
}

export async function addSkillRepo(url: string, branch?: string): Promise<SkillRepoChangeResult> {
  return invoke<SkillRepoChangeResult>('add_skill_repo', { url, branch: branch ?? null });
}

export async function removeSkillRepo(
  owner: string,
  name: string,
): Promise<SkillRepoChangeResult> {
  return invoke<SkillRepoChangeResult>('remove_skill_repo', { owner, name });
}

export async function searchSkillsSh(
  query: string,
  limit?: number,
  offset?: number,
): Promise<DiscoverableSkill[]> {
  return invoke<DiscoverableSkill[]>('search_skills_sh', {
    query,
    limit: limit ?? null,
    offset: offset ?? null,
  });
}

export async function getTargetSkillStates(targetId: string): Promise<SkillWithTargetState[]> {
  return invoke<SkillWithTargetState[]>('get_target_skill_states', { targetId });
}
