import { describe, expect, it } from 'vitest';
import { cleanupWarningsMessage } from '../utils/cleanupWarnings';
import type { AppState } from '../model/types';

function baseState(overrides: Partial<AppState> = {}): AppState {
  return {
    config: {
      version: 6,
      settings: {
        mainSkillsDir: null,
        linkStrategy: 'auto',
        startupRefresh: { github: false, gitlab: true, skillHub: true },
      },
      projects: [],
      targets: [],
      installations: [],
    },
    skills: [],
    selectedTargetId: null,
    selectedTargetSkills: [],
    ...overrides,
  };
}

describe('cleanupWarningsMessage', () => {
  it('returns null when warnings missing or empty', () => {
    expect(cleanupWarningsMessage(baseState())).toBeNull();
    expect(cleanupWarningsMessage(baseState({ cleanupWarnings: [] }))).toBeNull();
  });

  it('joins multiple warnings', () => {
    expect(
      cleanupWarningsMessage(
        baseState({ cleanupWarnings: ['a 请手动清理', 'b 请手动清理'] }),
      ),
    ).toBe('a 请手动清理；b 请手动清理');
  });
});
