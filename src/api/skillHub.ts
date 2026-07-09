import { invoke } from '@tauri-apps/api/core';
import type {
  DiscoverableSkill,
  DiscoverSkillsResult,
  PreviewAddRepoResult,
  SkillHubEndpoint,
  SkillHubEndpointChangeResult,
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

export async function discoverSkills(force = false): Promise<DiscoverSkillsResult> {
  return invoke<DiscoverSkillsResult>('discover_skills', { force });
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

export async function getTargetSkillStates(targetId: string): Promise<SkillWithTargetState[]> {
  return invoke<SkillWithTargetState[]>('get_target_skill_states', { targetId });
}

export async function listSkillHubEndpoints(): Promise<SkillHubEndpoint[]> {
  return invoke<SkillHubEndpoint[]>('list_skill_hub_endpoints');
}

export async function addSkillHubEndpoint(
  name: string,
  baseUrl: string,
): Promise<SkillHubEndpointChangeResult> {
  return invoke<SkillHubEndpointChangeResult>('add_skill_hub_endpoint', { name, baseUrl });
}

export async function removeSkillHubEndpoint(id: string): Promise<SkillHubEndpointChangeResult> {
  return invoke<SkillHubEndpointChangeResult>('remove_skill_hub_endpoint', { id });
}

export async function setSkillHubEndpointEnabled(
  id: string,
  enabled: boolean,
): Promise<SkillHubEndpointChangeResult> {
  return invoke<SkillHubEndpointChangeResult>('set_skill_hub_endpoint_enabled', { id, enabled });
}

export async function listHubGroups(hubEndpointId: string): Promise<string[]> {
  return invoke<string[]>('list_hub_groups', { hubEndpointId });
}

export async function createHubGroup(hubEndpointId: string, name: string): Promise<string[]> {
  return invoke<string[]>('create_hub_group', { hubEndpointId, name });
}

export async function uploadSkillToHub(
  hubEndpointId: string,
  group: string,
  storageKey: string,
): Promise<SkillHubEndpointChangeResult> {
  return invoke<SkillHubEndpointChangeResult>('upload_skill_to_hub', {
    hubEndpointId,
    group,
    storageKey,
  });
}
