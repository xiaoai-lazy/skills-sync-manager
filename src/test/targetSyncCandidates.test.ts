import { describe, expect, it } from 'vitest';
import type { AppConfig, Installation, Target } from '../model/types';
import {
  countInstallationsForTarget,
  defaultSyncSourceId,
  listSyncSourceCandidates,
  shouldOfferPostCreateSync,
} from '../utils/targetSyncCandidates';

function target(partial: Partial<Target> & Pick<Target, 'id' | 'name' | 'scope'>): Target {
  return {
    kind: 'custom',
    skillsDir: `/skills/${partial.id}`,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...partial,
  };
}

function installation(
  partial: Partial<Installation> & Pick<Installation, 'id' | 'targetId' | 'skillStorageKey'>,
): Installation {
  return {
    skillDirName: partial.skillStorageKey,
    skillName: partial.skillStorageKey,
    sourcePath: `/src/${partial.skillStorageKey}`,
    linkPath: `/link/${partial.id}`,
    linkType: 'junction',
    createdAt: '2026-01-01T00:00:00Z',
    ...partial,
  };
}

function baseConfig(overrides: Partial<AppConfig> = {}): AppConfig {
  return {
    version: 1,
    settings: {
      mainSkillsDir: null,
      linkStrategy: 'auto',
      startupRefresh: { github: false, gitlab: true, skillHub: true },
    },
    projects: [],
    targets: [],
    installations: [],
    ...overrides,
  };
}

describe('targetSyncCandidates', () => {
  const dest = target({
    id: 'dest',
    name: 'Dest',
    scope: 'project',
    projectId: 'proj-a',
  });

  const sameProjectAlpha = target({
    id: 'src-alpha',
    name: 'Alpha',
    scope: 'project',
    projectId: 'proj-a',
  });

  const sameProjectBravo = target({
    id: 'src-bravo',
    name: 'Bravo',
    scope: 'project',
    projectId: 'proj-a',
  });

  const sameProjectZero = target({
    id: 'src-zero',
    name: 'Zero',
    scope: 'project',
    projectId: 'proj-a',
  });

  const otherProject = target({
    id: 'src-other',
    name: 'Other',
    scope: 'project',
    projectId: 'proj-b',
  });

  const globalTarget = target({
    id: 'src-global',
    name: 'Global',
    scope: 'global',
  });

  const config = baseConfig({
    targets: [
      dest,
      sameProjectAlpha,
      sameProjectBravo,
      sameProjectZero,
      otherProject,
      globalTarget,
    ],
    installations: [
      installation({ id: 'i1', targetId: 'src-bravo', skillStorageKey: 'skill-a' }),
      installation({ id: 'i2', targetId: 'src-bravo', skillStorageKey: 'skill-b' }),
      installation({ id: 'i3', targetId: 'src-alpha', skillStorageKey: 'skill-c' }),
      installation({ id: 'i4', targetId: 'src-other', skillStorageKey: 'skill-d' }),
      installation({ id: 'i5', targetId: 'src-global', skillStorageKey: 'skill-e' }),
      installation({ id: 'i6', targetId: 'dest', skillStorageKey: 'skill-f' }),
    ],
  });

  it('countInstallationsForTarget counts only matching targetId', () => {
    expect(countInstallationsForTarget(config, 'src-bravo')).toBe(2);
    expect(countInstallationsForTarget(config, 'src-zero')).toBe(0);
  });

  it('excludes global targets, other projects, self, and zero installs', () => {
    const candidates = listSyncSourceCandidates(config, dest);
    expect(candidates.map((c) => c.target.id)).toEqual(['src-bravo', 'src-alpha']);
    expect(candidates.every((c) => c.installedCount > 0)).toBe(true);
  });

  it('returns empty for global or project-less dest', () => {
    expect(listSyncSourceCandidates(config, globalTarget)).toEqual([]);
    expect(
      listSyncSourceCandidates(
        config,
        target({ id: 'no-proj', name: 'NoProj', scope: 'project' }),
      ),
    ).toEqual([]);
  });

  it('sorts by installedCount desc then name asc', () => {
    const tieLow = target({
      id: 'tie-zulu',
      name: 'Zulu',
      scope: 'project',
      projectId: 'proj-a',
    });
    const tieHigh = target({
      id: 'tie-able',
      name: 'Able',
      scope: 'project',
      projectId: 'proj-a',
    });
    const sortedConfig = baseConfig({
      targets: [dest, sameProjectBravo, tieLow, tieHigh],
      installations: [
        installation({ id: 'b1', targetId: 'src-bravo', skillStorageKey: 's1' }),
        installation({ id: 'b2', targetId: 'src-bravo', skillStorageKey: 's2' }),
        installation({ id: 'z1', targetId: 'tie-zulu', skillStorageKey: 's3' }),
        installation({ id: 'a1', targetId: 'tie-able', skillStorageKey: 's4' }),
      ],
    });

    const candidates = listSyncSourceCandidates(sortedConfig, dest);
    expect(candidates.map((c) => ({ id: c.target.id, n: c.installedCount }))).toEqual([
      { id: 'src-bravo', n: 2 },
      { id: 'tie-able', n: 1 },
      { id: 'tie-zulu', n: 1 },
    ]);
  });

  it('defaultSyncSourceId returns first candidate or null', () => {
    const candidates = listSyncSourceCandidates(config, dest);
    expect(defaultSyncSourceId(candidates)).toBe('src-bravo');
    expect(defaultSyncSourceId([])).toBeNull();
  });

  it('shouldOfferPostCreateSync is false when no candidates', () => {
    const emptyOnly = baseConfig({
      targets: [dest, sameProjectZero, globalTarget, otherProject],
      installations: [
        installation({ id: 'o1', targetId: 'src-other', skillStorageKey: 's1' }),
        installation({ id: 'g1', targetId: 'src-global', skillStorageKey: 's2' }),
      ],
    });
    expect(shouldOfferPostCreateSync(emptyOnly, dest)).toBe(false);
    expect(shouldOfferPostCreateSync(config, undefined)).toBe(false);
    expect(shouldOfferPostCreateSync(config, dest)).toBe(true);
  });
});
