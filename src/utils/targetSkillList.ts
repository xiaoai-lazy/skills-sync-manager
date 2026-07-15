import type { SkillRecord, SkillWithTargetState } from '../model/types';
import {
  matchesInstalledNode,
  resolveSkillRecord,
} from '../components/skill-hub/sourceTreeUtils';

export function skillDisplayName(skill: {
  name: string | null;
  dirName: string;
}): string {
  const trimmed = skill.name?.trim();
  return trimmed ? trimmed : skill.dirName;
}

export function compareSkillDisplayName(
  a: { name: string | null; dirName: string },
  b: { name: string | null; dirName: string },
): number {
  return skillDisplayName(a).localeCompare(skillDisplayName(b), undefined, {
    sensitivity: 'base',
  });
}

export function sortTargetSkillRows(
  skills: SkillWithTargetState[],
): SkillWithTargetState[] {
  return [...skills].sort((a, b) => compareSkillDisplayName(a.skill, b.skill));
}

export function countTargetNodeSkills(
  nodeId: string,
  skills: SkillWithTargetState[],
  skillRecords: Record<string, SkillRecord>,
): { installed: number; total: number } {
  const inNode = skills.filter(
    (item) =>
      item.skill.valid &&
      matchesInstalledNode(
        nodeId,
        item.skill.dirName,
        resolveSkillRecord(item.skill, skillRecords),
        item.skill,
      ),
  );
  const installed = inNode.filter((item) => item.state === 'installed').length;
  return { installed, total: inNode.length };
}

export function formatNodeCount(installed: number, total: number): string {
  return `${installed}/${total}`;
}
