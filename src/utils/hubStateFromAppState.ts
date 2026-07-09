import type { AppState, SkillHubLocalState } from '../model/types';

export const emptyHubState: SkillHubLocalState = {
  skills: [],
  validCount: 0,
  invalidCount: 0,
  pendingUpdateCount: 0,
  lastScanAt: '',
  skillRecords: {},
};

export function hubStateFromAppState(state: AppState): SkillHubLocalState {
  const validCount = state.skills.filter((s) => s.valid).length;
  const invalidCount = state.skills.length - validCount;
  return {
    skills: state.skills,
    validCount,
    invalidCount,
    pendingUpdateCount: state.config.skillUpdateCache?.updates?.length ?? 0,
    lastScanAt: new Date().toISOString(),
    skillRecords: state.config.skillRecords ?? {},
  };
}
