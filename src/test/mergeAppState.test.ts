import { describe, expect, it } from 'vitest';
import { mergeAppState } from '../utils/mergeAppState';
import type { AppState, SkillView } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

function skill(dirName: string): SkillView {
  return {
    ...emptyV6SkillViewFields,
    dirName,
    name: dirName,
    description: null,
    path: `/skills/${dirName}`,
    valid: true,
    validationErrors: [],
    linkName: dirName,
  };
}

function baseState(overrides: Partial<AppState> = {}): AppState {
  return {
    config: {
      version: 1,
      settings: { mainSkillsDir: null, linkStrategy: 'auto' },
      targets: [],
      installations: [],
      projects: [],
      skillRepos: [],
      skillRecords: {},
      skillHubEndpoints: [],
    },
    skills: [],
    selectedTargetId: null,
    selectedTargetSkills: [],
    ...overrides,
  };
}

describe('mergeAppState', () => {
  it('keeps prev.skills when next.skillsIncluded is false', () => {
    const prev = baseState({
      skills: [skill('a')],
      skillsIncluded: true,
      selectedTargetSkills: [
        {
          skill: skill('a'),
          state: 'installed',
          message: null,
        },
      ],
    });
    const next = baseState({
      skills: [],
      skillsIncluded: false,
      selectedTargetSkills: [],
    });
    const merged = mergeAppState(prev, next);
    expect(merged.skills).toEqual(prev.skills);
    expect(merged.selectedTargetSkills).toEqual(prev.selectedTargetSkills);
  });

  it('uses next.selectedTargetSkills when light response includes them', () => {
    const prev = baseState({
      skills: [skill('a')],
      selectedTargetSkills: [
        { skill: skill('a'), state: 'installed', message: null },
      ],
    });
    const nextSkills = [
      { skill: skill('a'), state: 'notInstalled' as const, message: null },
    ];
    const next = baseState({
      skills: [],
      skillsIncluded: false,
      selectedTargetSkills: nextSkills,
    });
    expect(mergeAppState(prev, next).selectedTargetSkills).toEqual(nextSkills);
  });

  it('replaces skills when next.skillsIncluded is true or undefined', () => {
    const prev = baseState({ skills: [skill('a')] });
    const next = baseState({ skills: [skill('b')] });
    expect(mergeAppState(prev, next).skills).toEqual(next.skills);

    const nextExplicit = baseState({
      skills: [skill('c')],
      skillsIncluded: true,
    });
    expect(mergeAppState(prev, nextExplicit).skills).toEqual(nextExplicit.skills);
  });

  it('returns next when prev is null', () => {
    const next = baseState({ skills: [skill('x')] });
    expect(mergeAppState(null, next)).toBe(next);
  });
});
