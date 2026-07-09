import type { AppErrorDto } from '../model/types';

const IN_PROGRESS_CODES = new Set(['discoverInProgress', 'updatesInProgress']);

export function errorCode(err: unknown): string | null {
  if (!err || typeof err !== 'object') return null;
  const dto = err as Partial<AppErrorDto>;
  return typeof dto.code === 'string' ? dto.code : null;
}

export function isInProgressError(err: unknown): boolean {
  const code = errorCode(err);
  return code !== null && IN_PROGRESS_CODES.has(code);
}

export function isHubSkillGoneError(err: unknown): boolean {
  return errorCode(err) === 'hubSkillGone';
}
