import { describe, expect, it } from 'vitest';
import { hubStateFromAppState, emptyHubState } from '../utils/hubStateFromAppState';
import type { AppState } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

function baseState(overrides: Partial<AppState> = {}): AppState {
  return {
    config: {
      version: 1,
      settings: { mainSkillsDir: null, linkStrategy: 'auto' },
      targets: [],
      installations: [],
      projects: [],
      skillRepos: [],
      skillRecords: {
        'hub/a': {
          source: 'skillhub',
          storageKey: 'hub/a',
          linkName: 'a',
          repoHost: '',
          projectPath: '',
          repoOwner: '',
          repoName: '',
          repoBranch: '',
          directory: '',
          contentHash: '',
          installedAt: '',
          repoSlug: '',
          hubEndpointId: 'e1',
          hubSkillGroup: 'g',
          hubSkillId: 'a',
        },
      },
      skillHubEndpoints: [],
    },
    skills: [
      {
        ...emptyV6SkillViewFields,
        dirName: 'a',
        name: 'a',
        description: null,
        path: '/a',
        valid: true,
        validationErrors: [],
        linkName: 'a',
      },
      {
        ...emptyV6SkillViewFields,
        dirName: 'b',
        name: 'b',
        description: null,
        path: '/b',
        valid: false,
        validationErrors: ['bad'],
        linkName: 'b',
      },
    ],
    selectedTargetId: null,
    selectedTargetSkills: [],
    ...overrides,
  };
}

describe('hubStateFromAppState', () => {
  it('derives counts and records from app state', () => {
    const state = baseState({
      config: {
        ...baseState().config,
        skillUpdateCache: {
          checkedAt: 't',
          updates: [
            {
              dirName: 'a',
              storageKey: 'hub/a',
              currentHash: '1',
              remoteHash: '2',
              name: 'a',
            },
          ],
        },
      },
    });
    const hub = hubStateFromAppState(state);
    expect(hub.skills).toEqual(state.skills);
    expect(hub.validCount).toBe(1);
    expect(hub.invalidCount).toBe(1);
    expect(hub.pendingUpdateCount).toBe(1);
    expect(hub.skillRecords).toEqual(state.config.skillRecords);
    expect(hub.lastScanAt).toBeTruthy();
  });

  it('emptyHubState has zero counts', () => {
    expect(emptyHubState.validCount).toBe(0);
    expect(emptyHubState.skills).toEqual([]);
  });
});
