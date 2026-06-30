import { vi, describe, it, expect, beforeEach } from 'vitest';
import type { DiscoverableSkill } from '../model/types';

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
} from '../api/skillHub';

const sampleDiscoverable: DiscoverableSkill = {
  key: 'anthropics/skills:skills/brainstorming',
  name: 'brainstorming',
  description: 'Explore ideas before implementation.',
  directory: 'skills/brainstorming',
  installDirName: 'brainstorming',
  repoOwner: 'anthropics',
  repoName: 'skills',
  repoBranch: 'main',
  source: 'github',
};

beforeEach(() => {
  invokeMock.mockReset();
});

describe('skillHub API', () => {
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
    invokeMock.mockResolvedValue([sampleDiscoverable]);

    const result = await discoverSkills();

    expect(invokeMock).toHaveBeenCalledWith('discover_skills');
    expect(result).toEqual([sampleDiscoverable]);
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
});
