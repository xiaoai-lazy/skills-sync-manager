import type { AppState } from '../model/types';

export function mergeAppState(prev: AppState | null, next: AppState): AppState {
  if (!prev) return next;
  if (next.skillsIncluded === false) {
    return {
      ...next,
      skills: prev.skills,
      selectedTargetSkills:
        next.selectedTargetSkills.length > 0
          ? next.selectedTargetSkills
          : prev.selectedTargetSkills,
    };
  }
  return next;
}
