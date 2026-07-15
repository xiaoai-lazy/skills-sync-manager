import { describe, expect, it } from 'vitest';
import { emptyV6SkillViewFields, type SkillWithTargetState } from '../model/types';
import {
  compareSkillDisplayName,
  countTargetNodeSkills,
  formatNodeCount,
  skillDisplayName,
  sortTargetSkillRows,
} from '../utils/targetSkillList';
import { ALL_NODE_ID, LOCAL_NODE_ID } from '../components/skill-hub/sourceTreeUtils';

function skillRow(
  overrides: Partial<SkillWithTargetState['skill']> & {
    state?: SkillWithTargetState['state'];
  },
): SkillWithTargetState {
  const { state = 'notInstalled', ...skillOverrides } = overrides;
  const dirName = skillOverrides.dirName ?? 'skill';
  return {
    skill: {
      dirName,
      name: skillOverrides.name ?? dirName,
      description: skillOverrides.description ?? null,
      path: skillOverrides.path ?? `/tmp/${dirName}`,
      valid: skillOverrides.valid ?? true,
      validationErrors: skillOverrides.validationErrors ?? [],
      ...emptyV6SkillViewFields,
      storageKey: skillOverrides.storageKey ?? `local/${dirName}`,
      linkName: skillOverrides.linkName ?? dirName,
      ...skillOverrides,
    },
    state,
    message: null,
  };
}

describe('skillDisplayName', () => {
  it('returns trimmed name when present', () => {
    expect(skillDisplayName({ name: '  Alpha  ', dirName: 'alpha' })).toBe('Alpha');
  });

  it('falls back to dirName when name is null or blank', () => {
    expect(skillDisplayName({ name: null, dirName: 'fallback-dir' })).toBe('fallback-dir');
    expect(skillDisplayName({ name: '   ', dirName: 'fallback-dir' })).toBe('fallback-dir');
  });
});

describe('compareSkillDisplayName', () => {
  it('compares with base sensitivity', () => {
    expect(
      compareSkillDisplayName(
        { name: 'Alpha', dirName: 'a' },
        { name: 'alpha', dirName: 'b' },
      ),
    ).toBe(0);
  });
});

describe('sortTargetSkillRows', () => {
  it('sorts by display name regardless of install state', () => {
    const rows = [
      skillRow({ dirName: 'alpha', name: 'alpha', state: 'notInstalled' }),
      skillRow({ dirName: 'zebra', name: 'zebra', state: 'installed' }),
      skillRow({ dirName: 'beta', name: 'beta', state: 'installed' }),
      skillRow({ dirName: 'gamma', name: 'gamma', state: 'conflict' }),
    ];

    const sorted = sortTargetSkillRows(rows);
    expect(sorted.map((r) => r.skill.dirName)).toEqual([
      'alpha',
      'beta',
      'gamma',
      'zebra',
    ]);
  });

  it('does not mutate the input array', () => {
    const rows = [
      skillRow({ dirName: 'b', state: 'notInstalled' }),
      skillRow({ dirName: 'a', state: 'installed' }),
    ];
    const copy = [...rows];
    sortTargetSkillRows(rows);
    expect(rows).toEqual(copy);
  });
});

describe('countTargetNodeSkills', () => {
  it('counts only valid skills matching the node', () => {
    const skills = [
      skillRow({ dirName: 'a', state: 'installed', valid: true }),
      skillRow({ dirName: 'b', state: 'notInstalled', valid: true }),
      skillRow({ dirName: 'c', state: 'installed', valid: false }),
    ];

    expect(countTargetNodeSkills(ALL_NODE_ID, skills, {})).toEqual({
      installed: 1,
      total: 2,
    });
  });

  it('filters by node for local installs', () => {
    const skills = [
      skillRow({
        dirName: 'local-one',
        storageKey: 'local/local-one',
        state: 'installed',
      }),
      skillRow({
        dirName: 'hub-one',
        storageKey: 'hub/ep/common/hub-one',
        state: 'installed',
      }),
    ];
    const skillRecords = {
      'hub/ep/common/hub-one': {
        source: 'skillhub',
        storageKey: 'hub/ep/common/hub-one',
        linkName: 'hub-one',
        hubEndpointId: 'ep',
        hubSkillGroup: 'common',
        hubSkillId: 'hub-one',
        repoHost: '',
        projectPath: '',
        repoOwner: '',
        repoName: '',
        repoBranch: '',
        directory: 'common/hub-one',
        contentHash: '',
        installedAt: '',
        repoSlug: '',
      },
    };

    expect(countTargetNodeSkills(LOCAL_NODE_ID, skills, skillRecords)).toEqual({
      installed: 1,
      total: 1,
    });
  });
});

describe('formatNodeCount', () => {
  it('formats installed/total', () => {
    expect(formatNodeCount(1, 2)).toBe('1/2');
    expect(formatNodeCount(0, 0)).toBe('0/0');
  });
});
