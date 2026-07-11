import type { AppState } from '../model/types';

/** Apply force-cleanup soft warnings to the UI error/toast channel. */
export function cleanupWarningsMessage(state: AppState): string | null {
  const warnings = state.cleanupWarnings;
  if (!warnings || warnings.length === 0) return null;
  return warnings.length === 1 ? warnings[0] : warnings.join('；');
}
