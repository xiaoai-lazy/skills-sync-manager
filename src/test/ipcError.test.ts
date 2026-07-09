import { describe, expect, it } from 'vitest';
import { errorCode, isInProgressError, isHubSkillGoneError } from '../utils/ipcError';

describe('ipcError', () => {
  it('detects discoverInProgress and updatesInProgress', () => {
    expect(isInProgressError({ code: 'discoverInProgress', message: 'busy' })).toBe(true);
    expect(isInProgressError({ code: 'updatesInProgress', message: 'busy' })).toBe(true);
  });

  it('rejects other errors', () => {
    expect(isInProgressError({ code: 'conflict', message: 'x' })).toBe(false);
    expect(isInProgressError(new Error('fail'))).toBe(false);
    expect(isInProgressError('string')).toBe(false);
    expect(isInProgressError(null)).toBe(false);
  });

  it('extracts error code', () => {
    expect(errorCode({ code: 'discoverInProgress', message: 'x' })).toBe('discoverInProgress');
    expect(errorCode({})).toBeNull();
  });

  it('detects hubSkillGone', () => {
    expect(isHubSkillGoneError({ code: 'hubSkillGone', message: 'gone' })).toBe(true);
    expect(isHubSkillGoneError({ code: 'downloadFailed', message: 'x' })).toBe(false);
  });
});
