import { invoke } from '@tauri-apps/api/core';
import type {
  DiscoverableSkill,
  DiscoverSkillsResult,
  PreviewAddRepoResult,
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

export async function discoverSkills(): Promise<DiscoverSkillsResult> {
  return invoke<DiscoverSkillsResult>('discover_skills');
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

export async function previewAddSkillRepo(url: string): Promise<PreviewAddRepoResult> {
  return invoke<PreviewAddRepoResult>('preview_add_skill_repo', { url });
}

export async function validateGitlabPat(host: string, pat: string): Promise<void> {
  return invoke<void>('validate_gitlab_pat', { host, pat });
}

export async function listGitlabCredentials(): Promise<string[]> {
  return invoke<string[]>('list_gitlab_credentials');
}

export async function removeGitlabCredential(host: string): Promise<void> {
  return invoke<void>('remove_gitlab_credential', { host });
}

export async function updateGitlabCredential(host: string, pat: string): Promise<void> {
  return invoke<void>('update_gitlab_credential', { host, pat });
}

export async function addSkillRepo(
  url: string,
  branch?: string,
  pat?: string,
): Promise<SkillRepoChangeResult> {
  return invoke<SkillRepoChangeResult>('add_skill_repo', {
    url,
    branch: branch ?? null,
    pat: pat ?? null,
  });
}

export async function removeSkillRepo(
  host: string,
  projectPath: string,
): Promise<SkillRepoChangeResult> {
  return invoke<SkillRepoChangeResult>('remove_skill_repo', { host, projectPath });
}

export async function setSkillRepoEnabled(
  host: string,
  projectPath: string,
  enabled: boolean,
): Promise<SkillRepoChangeResult> {
  return invoke<SkillRepoChangeResult>('set_skill_repo_enabled', {
    host,
    projectPath,
    enabled,
  });
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
