import { vi, describe, it, expect, beforeEach } from 'vitest';
import type { DiscoverableSkill } from '../model/types';
import { emptyV6DiscoverableFields } from '../model/types';

const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import {
  scanMainLibrary,
  discoverSkills,
  installHubSkill,
  parseSmartPaste,
  getTargetSkillStates,
  previewAddSkillRepo,
  validateGitlabPat,
  listGitlabCredentials,
  removeGitlabCredential,
  updateGitlabCredential,
  addSkillRepo,
  removeSkillRepo,
  setSkillRepoEnabled,
  listSkillHubEndpoints,
  addSkillHubEndpoint,
  removeSkillHubEndpoint,
  setSkillHubEndpointEnabled,
  listHubGroups,
  createHubGroup,
  uploadSkillToHub,
  refreshStartupSkillSources,
  setStartupRefreshSettings,
} from '../api/skillHub';

const sampleDiscoverable: DiscoverableSkill = {
  key: 'anthropics/skills:skills/brainstorming',
  name: 'brainstorming',
  description: 'Explore ideas before implementation.',
  directory: 'skills/brainstorming',
  installDirName: 'brainstorming',
  repoHost: 'github.com',
  projectPath: 'anthropics/skills',
  repoOwner: 'anthropics',
  repoName: 'skills',
  repoBranch: 'main',
  source: 'github',
  ...emptyV6DiscoverableFields,
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe('skillHub API', () => {
  it('refreshStartupSkillSources invokes the startup-only command', async () => {
    invokeMock.mockResolvedValue({ discoverSkills: [], pendingUpdates: [], warnings: [] });

    await refreshStartupSkillSources();

    expect(invokeMock).toHaveBeenCalledWith('refresh_startup_skill_sources');
  });

  it('setStartupRefreshSettings sends all source switches', async () => {
    const settings = { github: true, gitlab: false, skillHub: true };
    invokeMock.mockResolvedValue(settings);

    await setStartupRefreshSettings(settings);

    expect(invokeMock).toHaveBeenCalledWith('set_startup_refresh_settings', { settings });
  });

  it('scanMainLibrary calls invoke with scan_main_library', async () => {
    invokeMock.mockResolvedValue({
      skills: [],
      validCount: 0,
      invalidCount: 0,
      pendingUpdateCount: 0,
      lastScanAt: '2026-06-30T00:00:00Z',
      skillRecords: {},
    });

    await scanMainLibrary();

    expect(invokeMock).toHaveBeenCalledWith('scan_main_library');
  });

  it('installHubSkill passes discoverable skill payload', async () => {
    invokeMock.mockResolvedValue({
      skills: [],
      validCount: 0,
      invalidCount: 0,
      pendingUpdateCount: 0,
      lastScanAt: '2026-06-30T00:00:00Z',
      skillRecords: {
        brainstorming: {
          source: 'github',
          repoOwner: 'anthropics',
          repoName: 'skills',
          repoBranch: 'main',
          directory: 'skills/brainstorming',
          contentHash: 'hash',
          installedAt: '2026-06-30T00:00:00Z',
        },
      },
    });

    const result = await installHubSkill(sampleDiscoverable);

    expect(invokeMock).toHaveBeenCalledWith('install_hub_skill', {
      discoverable: sampleDiscoverable,
    });
    expect(result.skillRecords.brainstorming?.source).toBe('github');
  });

  it('discoverSkills calls invoke with discover_skills', async () => {
    invokeMock.mockResolvedValue({ skills: [sampleDiscoverable], warnings: [] });

    const result = await discoverSkills();

    expect(invokeMock).toHaveBeenCalledWith('discover_skills', { force: false });
    expect(result.skills).toEqual([sampleDiscoverable]);
    expect(result.warnings).toEqual([]);
  });

  it('parseSmartPaste passes input to parse_smart_paste', async () => {
    const preview = {
      name: 'brainstorming',
      description: 'Explore ideas.',
      installDirName: 'brainstorming',
      repoOwner: 'anthropics',
      repoName: 'skills',
      repoBranch: 'main',
      directory: 'skills/brainstorming',
      source: 'github',
    };
    invokeMock.mockResolvedValue(preview);

    const result = await parseSmartPaste(
      'https://github.com/anthropics/skills/tree/main/skills/brainstorming',
    );

    expect(invokeMock).toHaveBeenCalledWith('parse_smart_paste', {
      input: 'https://github.com/anthropics/skills/tree/main/skills/brainstorming',
    });
    expect(result).toEqual(preview);
  });

  it('getTargetSkillStates passes targetId to get_target_skill_states', async () => {
    invokeMock.mockResolvedValue([]);

    await getTargetSkillStates('target_1');

    expect(invokeMock).toHaveBeenCalledWith('get_target_skill_states', {
      targetId: 'target_1',
    });
  });

  it('previewAddSkillRepo passes url to preview_add_skill_repo', async () => {
    invokeMock.mockResolvedValue({
      canSave: true,
      needsPat: false,
      host: 'github.com',
      provider: 'github',
      projectPath: 'anthropics/skills',
      branch: 'main',
      error: null,
    });

    const result = await previewAddSkillRepo('https://github.com/anthropics/skills');

    expect(invokeMock).toHaveBeenCalledWith('preview_add_skill_repo', {
      url: 'https://github.com/anthropics/skills',
    });
    expect(result.canSave).toBe(true);
  });

  it('validateGitlabPat passes host and pat to validate_gitlab_pat', async () => {
    invokeMock.mockResolvedValue(undefined);

    await validateGitlabPat('gitlab.example.com', 'glpat-test');

    expect(invokeMock).toHaveBeenCalledWith('validate_gitlab_pat', {
      host: 'gitlab.example.com',
      pat: 'glpat-test',
    });
  });

  it('listGitlabCredentials calls list_gitlab_credentials', async () => {
    invokeMock.mockResolvedValue(['gitlab.example.com']);

    const result = await listGitlabCredentials();

    expect(invokeMock).toHaveBeenCalledWith('list_gitlab_credentials');
    expect(result).toEqual(['gitlab.example.com']);
  });

  it('removeGitlabCredential passes host to remove_gitlab_credential', async () => {
    invokeMock.mockResolvedValue(undefined);

    await removeGitlabCredential('gitlab.example.com');

    expect(invokeMock).toHaveBeenCalledWith('remove_gitlab_credential', {
      host: 'gitlab.example.com',
    });
  });

  it('updateGitlabCredential passes host and pat to update_gitlab_credential', async () => {
    invokeMock.mockResolvedValue(undefined);

    await updateGitlabCredential('gitlab.example.com', 'glpat-new');

    expect(invokeMock).toHaveBeenCalledWith('update_gitlab_credential', {
      host: 'gitlab.example.com',
      pat: 'glpat-new',
    });
  });

  it('addSkillRepo passes url branch and pat to add_skill_repo', async () => {
    invokeMock.mockResolvedValue({ repos: [], discoverSkills: [] });

    await addSkillRepo('https://gitlab.example.com/acme/tools', 'main', 'glpat-test');

    expect(invokeMock).toHaveBeenCalledWith('add_skill_repo', {
      url: 'https://gitlab.example.com/acme/tools',
      branch: 'main',
      pat: 'glpat-test',
    });
  });

  it('removeSkillRepo passes host and projectPath to remove_skill_repo', async () => {
    invokeMock.mockResolvedValue({ repos: [], discoverSkills: [] });

    await removeSkillRepo('gitlab.example.com', 'acme/tools');

    expect(invokeMock).toHaveBeenCalledWith('remove_skill_repo', {
      host: 'gitlab.example.com',
      projectPath: 'acme/tools',
    });
  });

  it('setSkillRepoEnabled passes host projectPath and enabled', async () => {
    invokeMock.mockResolvedValue({ repos: [], discoverSkills: [] });

    await setSkillRepoEnabled('gitlab.example.com', 'acme/tools', false);

    expect(invokeMock).toHaveBeenCalledWith('set_skill_repo_enabled', {
      host: 'gitlab.example.com',
      projectPath: 'acme/tools',
      enabled: false,
    });
  });

  it('listSkillHubEndpoints calls list_skill_hub_endpoints', async () => {
    invokeMock.mockResolvedValue([]);

    await listSkillHubEndpoints();

    expect(invokeMock).toHaveBeenCalledWith('list_skill_hub_endpoints');
  });

  it('addSkillHubEndpoint passes name and baseUrl', async () => {
    invokeMock.mockResolvedValue({ endpoints: [], discoverSkills: [] });

    await addSkillHubEndpoint('Company Hub', 'https://hub.example.com');

    expect(invokeMock).toHaveBeenCalledWith('add_skill_hub_endpoint', {
      name: 'Company Hub',
      baseUrl: 'https://hub.example.com',
    });
  });

  it('removeSkillHubEndpoint passes id', async () => {
    invokeMock.mockResolvedValue({ endpoints: [], discoverSkills: [] });

    await removeSkillHubEndpoint('hub-1');

    expect(invokeMock).toHaveBeenCalledWith('remove_skill_hub_endpoint', { id: 'hub-1' });
  });

  it('setSkillHubEndpointEnabled passes id and enabled', async () => {
    invokeMock.mockResolvedValue({ endpoints: [], discoverSkills: [] });

    await setSkillHubEndpointEnabled('hub-1', false);

    expect(invokeMock).toHaveBeenCalledWith('set_skill_hub_endpoint_enabled', {
      id: 'hub-1',
      enabled: false,
    });
  });

  it('listHubGroups passes hubEndpointId', async () => {
    invokeMock.mockResolvedValue(['default']);

    const result = await listHubGroups('hub-1');

    expect(invokeMock).toHaveBeenCalledWith('list_hub_groups', { hubEndpointId: 'hub-1' });
    expect(result).toEqual(['default']);
  });

  it('createHubGroup passes hubEndpointId and name', async () => {
    invokeMock.mockResolvedValue(['default', 'new-group']);

    await createHubGroup('hub-1', 'new-group');

    expect(invokeMock).toHaveBeenCalledWith('create_hub_group', {
      hubEndpointId: 'hub-1',
      name: 'new-group',
    });
  });

  it('uploadSkillToHub passes hubEndpointId group and storageKey', async () => {
    invokeMock.mockResolvedValue({ endpoints: [], discoverSkills: [] });

    await uploadSkillToHub('hub-1', 'tools', 'github.com--owner/repo/skills/foo');

    expect(invokeMock).toHaveBeenCalledWith('upload_skill_to_hub', {
      hubEndpointId: 'hub-1',
      group: 'tools',
      storageKey: 'github.com--owner/repo/skills/foo',
    });
  });
});
