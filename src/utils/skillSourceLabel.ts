import { resolveSkillRecord } from '../components/skill-hub/sourceTreeUtils';
import type { DiscoverableSkill, SkillRecord, SkillView } from '../model/types';

type SourceLabelMeta = Partial<Pick<SkillRecord, 'repoHost' | 'hubSkillGroup'>>;

export function formatSkillSourceLabel(
  source: string,
  record?: SourceLabelMeta,
): string {
  if (source === 'skillhub') {
    return record?.hubSkillGroup
      ? `Skills Sync Hub · ${record.hubSkillGroup}`
      : 'Skills Sync Hub';
  }
  if (source === 'iflytek') {
    return record?.hubSkillGroup
      ? `iFlytek Skill Hub · ${record.hubSkillGroup}`
      : 'iFlytek Skill Hub';
  }
  if (source === 'skillssh') return 'GitHub（旧来源）';
  if (source === 'github') return 'GitHub';
  if (source === 'gitlab') {
    return record?.repoHost ? `GitLab · ${record.repoHost}` : 'GitLab';
  }
  return '本地导入';
}

export function skillSourceLabelForView(
  skill: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
  skillRecords?: Record<string, SkillRecord>,
): string {
  const record = resolveSkillRecord(skill, skillRecords);
  if (!record) return '本地导入';
  return formatSkillSourceLabel(record.source, record);
}

export function skillSourceLabelForDiscoverable(
  skill: Pick<DiscoverableSkill, 'source' | 'repoHost' | 'hubSkillGroup'>,
): string {
  return formatSkillSourceLabel(skill.source, {
    repoHost: skill.repoHost,
    hubSkillGroup: skill.hubSkillGroup,
  });
}
