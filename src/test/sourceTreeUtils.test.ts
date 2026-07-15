import { describe, expect, it } from 'vitest';
import {
  ALL_HUB_GROUP,
  dedupeInstalledSkills,
  findPendingUpdate,
  hubEndpointVisible,
  matchedPendingUpdates,
  matchesInstalledNode,
  resolveEffectiveFilterNodeId,
  resolveSkillRecord,
  skillHasPendingUpdate,
} from '../components/skill-hub/sourceTreeUtils';
import type { SkillHubEndpoint, SkillRecord, SkillUpdateInfo, SkillView } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

const hubTalos: SkillView = {
  ...emptyV6SkillViewFields,
  dirName: 'talos-lecture-json-review',
  name: 'talos-lecture-json-review',
  description: 'Hub copy',
  path: 'C:\\skills\\hub\\oxygen-skill-hub\\common\\talos-lecture-json-review',
  valid: true,
  validationErrors: [],
  storageKey: 'hub/oxygen-skill-hub/common/talos-lecture-json-review',
  linkName: 'talos-lecture-json-review',
};

const repoTalos: SkillView = {
  ...emptyV6SkillViewFields,
  dirName: 'talos-lecture-json-review',
  name: 'talos-lecture-json-review',
  description: 'Repo copy',
  path: 'C:\\skills\\repo\\git.xkw.cn--mp-oxygen-uc-skills\\talos-lecture-json-review',
  valid: true,
  validationErrors: [],
  storageKey: 'repo/git.xkw.cn--mp-oxygen-uc-skills/talos-lecture-json-review',
  linkName: 'talos-lecture-json-review',
};

const skillRecords: Record<string, SkillRecord> = {
  [hubTalos.storageKey]: {
    source: 'skillhub',
    storageKey: hubTalos.storageKey,
    linkName: 'talos-lecture-json-review',
    hubEndpointId: 'oxygen-skill-hub',
    hubSkillGroup: 'common',
    hubSkillId: 'talos-lecture-json-review',
    repoHost: '',
    projectPath: '',
    repoOwner: '',
    repoName: '',
    repoBranch: '',
    directory: 'common/talos-lecture-json-review',
    contentHash: '',
    installedAt: '',
    repoSlug: '',
  },
  [repoTalos.storageKey]: {
    source: 'gitlab',
    storageKey: repoTalos.storageKey,
    linkName: 'talos-lecture-json-review',
    hubEndpointId: '',
    hubSkillGroup: '',
    hubSkillId: '',
    repoHost: 'git.xkw.cn',
    projectPath: 'mp-oxygen/uc/skills',
    repoOwner: 'mp-oxygen/uc',
    repoName: 'skills',
    repoBranch: 'main',
    directory: 'skills/talos-lecture-json-review',
    contentHash: '',
    installedAt: '',
    repoSlug: 'git.xkw.cn--mp-oxygen-uc-skills',
  },
};

describe('sourceTreeUtils hub grouping', () => {
  const endpoints: SkillHubEndpoint[] = [
    {
      id: 'oxygen-skill-hub',
      name: 'Oxygen Skill Hub',
      baseUrl: 'http://localhost:3337',
      enabled: true,
    },
  ];

  it('resolveEffectiveFilterNodeId applies hub group filter on hub root', () => {
    expect(
      resolveEffectiveFilterNodeId('hub:oxygen-skill-hub', ALL_HUB_GROUP, endpoints),
    ).toBe('hub:oxygen-skill-hub');
    expect(
      resolveEffectiveFilterNodeId('hub:oxygen-skill-hub', 'common', endpoints),
    ).toBe('hub:oxygen-skill-hub:common');
  });

  it('shows only hub copy under hub common node', () => {
    const hubNode = 'hub:oxygen-skill-hub:common';

    expect(
      matchesInstalledNode(
        hubNode,
        hubTalos.dirName,
        resolveSkillRecord(hubTalos, skillRecords),
        hubTalos,
      ),
    ).toBe(true);

    expect(
      matchesInstalledNode(
        hubNode,
        repoTalos.dirName,
        resolveSkillRecord(repoTalos, skillRecords),
        repoTalos,
      ),
    ).toBe(false);
  });

  it('does not resolve repo skill to hub record when link names collide', () => {
    const record = resolveSkillRecord(repoTalos, skillRecords);
    expect(record?.source).toBe('gitlab');
    expect(record?.storageKey).toBe(repoTalos.storageKey);
  });

  it('dedupes installed skills by storageKey', () => {
    const deduped = dedupeInstalledSkills([hubTalos, hubTalos, repoTalos]);
    expect(deduped).toHaveLength(2);
  });

  it('resolveSkillRecord falls back to dirName only for legacy flat keys', () => {
    const legacyRecords: Record<string, SkillRecord> = {
      brainstorming: {
        ...skillRecords[hubTalos.storageKey],
        storageKey: 'brainstorming',
        linkName: 'brainstorming',
        source: 'local',
      },
    };
    const skill: SkillView = {
      ...hubTalos,
      storageKey: 'local/brainstorming',
      dirName: 'brainstorming',
      linkName: 'brainstorming',
    };
    expect(resolveSkillRecord(skill, legacyRecords)?.storageKey).toBe('brainstorming');
  });

  it('hubEndpointVisible keeps configured hubs without installs', () => {
    const endpoint: SkillHubEndpoint = {
      id: 'company-hub',
      name: 'Company Hub',
      baseUrl: 'https://hub.example.com',
      enabled: true,
    };
    expect(hubEndpointVisible(endpoint, {})).toBe(true);
    expect(hubEndpointVisible({ ...endpoint, enabled: false }, {})).toBe(true);
  });
});

describe('pending update matching', () => {
  const skill = hubTalos;
  const exact: SkillUpdateInfo = {
    dirName: skill.linkName,
    name: skill.name ?? skill.dirName,
    remoteHash: 'remote',
    storageKey: skill.storageKey,
  };

  it('matches by full storageKey', () => {
    expect(findPendingUpdate(skill, [exact])?.storageKey).toBe(skill.storageKey);
    expect(skillHasPendingUpdate(skill, [exact])).toBe(true);
  });

  it('matches legacy bare-name storageKey against nested skill key', () => {
    const bare: SkillUpdateInfo = {
      dirName: skill.linkName,
      name: skill.name ?? skill.dirName,
      remoteHash: 'remote',
      storageKey: skill.linkName,
    };
    expect(findPendingUpdate(skill, [bare])?.storageKey).toBe(skill.linkName);
  });

  it('matches empty storageKey via dirName', () => {
    const emptyKey: SkillUpdateInfo = {
      dirName: skill.linkName,
      name: skill.name ?? skill.dirName,
      remoteHash: 'remote',
      storageKey: '',
    };
    expect(findPendingUpdate(skill, [emptyKey])).toEqual(emptyKey);
  });

  it('matchedPendingUpdates ignores orphan cache entries', () => {
    const orphan: SkillUpdateInfo = {
      dirName: 'gone-skill',
      name: 'gone',
      remoteHash: 'x',
      storageKey: 'repo/gone/gone-skill',
    };
    expect(matchedPendingUpdates([skill], [exact, orphan])).toEqual([exact]);
    expect(matchedPendingUpdates([skill], [orphan])).toEqual([]);
  });
});
