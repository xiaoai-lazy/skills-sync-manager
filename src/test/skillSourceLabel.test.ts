import { describe, expect, it } from 'vitest';
import {
  formatSkillSourceLabel,
  skillSourceLabelForView,
} from '../utils/skillSourceLabel';
import type { SkillRecord } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

const gitlabRecord: SkillRecord = {
  repoHost: 'git.xkw.cn',
  projectPath: 'mp-oxygen/uc/skills',
  source: 'gitlab',
  repoOwner: 'mp-oxygen/uc',
  repoName: 'skills',
  repoBranch: 'master',
  directory: 'skills/foo',
  contentHash: 'abc',
  installedAt: '2026-01-01T00:00:00Z',
  storageKey: 'repo/git.xkw.cn--mp-oxygen-uc-skills/foo',
  linkName: 'foo',
  repoSlug: 'git.xkw.cn--mp-oxygen-uc-skills',
  hubEndpointId: '',
  hubSkillGroup: '',
  hubSkillId: '',
};

describe('skillSourceLabel', () => {
  it('formats Skills Sync Hub source with group', () => {
    expect(formatSkillSourceLabel('skillhub', { hubSkillGroup: 'common' })).toBe(
      'Skills Sync Hub · common',
    );
  });

  it('formats iFlytek Skill Hub source with namespace', () => {
    expect(formatSkillSourceLabel('iflytek', { hubSkillGroup: 'global' })).toBe(
      'iFlytek Skill Hub · global',
    );
  });

  it('formats Skills Sync Hub and iFlytek without group', () => {
    expect(formatSkillSourceLabel('skillhub')).toBe('Skills Sync Hub');
    expect(formatSkillSourceLabel('iflytek')).toBe('iFlytek Skill Hub');
  });

  it('formats gitlab source with host', () => {
    expect(formatSkillSourceLabel('gitlab', { repoHost: 'git.xkw.cn' })).toBe(
      'GitLab · git.xkw.cn',
    );
  });

  it('formats github source', () => {
    expect(formatSkillSourceLabel('github')).toBe('GitHub');
  });

  it('formats legacy skillssh source', () => {
    expect(formatSkillSourceLabel('skillssh')).toBe('GitHub（旧来源）');
  });

  it('resolves source label from skill record', () => {
    const label = skillSourceLabelForView(
      {
        ...emptyV6SkillViewFields,
        dirName: 'foo',
        storageKey: gitlabRecord.storageKey,
        linkName: 'foo',
      },
      { [gitlabRecord.storageKey]: gitlabRecord },
    );
    expect(label).toBe('GitLab · git.xkw.cn');
  });
});
