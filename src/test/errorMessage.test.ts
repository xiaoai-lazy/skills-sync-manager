import { describe, it, expect } from 'vitest';
import { errorMessage } from '../utils/errorMessage';

describe('errorMessage', () => {
  it('returns string errors as-is', () => {
    expect(errorMessage('目录已存在')).toBe('目录已存在');
  });

  it('returns Error message', () => {
    expect(errorMessage(new Error('网络失败'))).toBe('网络失败');
  });

  it('extracts message from Tauri AppErrorDto object', () => {
    expect(
      errorMessage({ code: 'dir_exists', message: '目标目录已存在' }),
    ).toBe('目标目录已存在');
  });

  it('does not stringify plain objects as [object Object]', () => {
    expect(errorMessage({ code: 'dir_exists', message: '目标目录已存在' })).not.toBe(
      '[object Object]',
    );
  });
});
