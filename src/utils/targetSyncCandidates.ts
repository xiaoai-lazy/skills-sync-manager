import type { AppConfig, Target } from '../model/types';

export interface SyncSourceCandidate {
  target: Target;
  installedCount: number;
}

export function countInstallationsForTarget(config: AppConfig, targetId: string): number {
  return config.installations.filter((i) => i.targetId === targetId).length;
}

export function listSyncSourceCandidates(
  config: AppConfig,
  destTarget: Target,
): SyncSourceCandidate[] {
  if (destTarget.scope !== 'project' || !destTarget.projectId) return [];
  return config.targets
    .filter(
      (t) =>
        t.id !== destTarget.id &&
        t.scope === 'project' &&
        t.projectId === destTarget.projectId,
    )
    .map((target) => ({
      target,
      installedCount: countInstallationsForTarget(config, target.id),
    }))
    .filter((c) => c.installedCount > 0)
    .sort((a, b) => {
      if (b.installedCount !== a.installedCount) {
        return b.installedCount - a.installedCount;
      }
      return a.target.name.localeCompare(b.target.name, undefined, {
        sensitivity: 'base',
      });
    });
}

export function defaultSyncSourceId(candidates: SyncSourceCandidate[]): string | null {
  return candidates[0]?.target.id ?? null;
}

export function shouldOfferPostCreateSync(
  config: AppConfig,
  destTarget: Target | undefined,
): boolean {
  if (!destTarget) return false;
  return listSyncSourceCandidates(config, destTarget).length > 0;
}
