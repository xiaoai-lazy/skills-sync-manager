import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://localhost/${encodeURIComponent(path)}`),
}));

import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { presetIconSrc } from '../utils/presetIconSrc';

describe('presetIconSrc', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReset();
    vi.mocked(convertFileSrc).mockClear();
  });

  it('returns undefined when iconUrl is missing', () => {
    expect(presetIconSrc()).toBeUndefined();
    expect(presetIconSrc('')).toBeUndefined();
  });

  it('returns raw path outside Tauri', () => {
    vi.mocked(isTauri).mockReturnValue(false);
    expect(presetIconSrc('C:\\icons\\cursor.png')).toBe('C:\\icons\\cursor.png');
    expect(convertFileSrc).not.toHaveBeenCalled();
  });

  it('converts local path through asset protocol in Tauri', () => {
    vi.mocked(isTauri).mockReturnValue(true);
    const path = 'C:\\icons\\cursor.png';
    expect(presetIconSrc(path)).toBe(`asset://localhost/${encodeURIComponent(path)}`);
    expect(convertFileSrc).toHaveBeenCalledWith(path);
  });
});
