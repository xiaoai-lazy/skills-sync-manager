import { describe, expect, it, vi, afterEach } from 'vitest';
import { isMacOS } from '../utils/platform';

describe('isMacOS', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('detects mac from navigator.platform', () => {
    vi.stubGlobal('navigator', { platform: 'MacIntel', userAgent: 'Mozilla/5.0' });
    expect(isMacOS()).toBe(true);
  });

  it('detects windows as non-mac', () => {
    vi.stubGlobal('navigator', {
      platform: 'Win32',
      userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)',
    });
    expect(isMacOS()).toBe(false);
  });

  it('detects mac from userAgentData.platform', () => {
    vi.stubGlobal('navigator', {
      platform: '',
      userAgent: 'Mozilla/5.0',
      userAgentData: { platform: 'macOS' },
    });
    expect(isMacOS()).toBe(true);
  });
});
