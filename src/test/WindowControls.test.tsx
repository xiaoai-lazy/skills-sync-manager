import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const minimize = vi.fn();
const toggleMaximize = vi.fn();
const close = vi.fn();
const isMacOSMock = vi.fn(() => false);

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize,
    toggleMaximize,
    close,
  }),
}));

vi.mock('../utils/platform', () => ({
  isMacOS: () => isMacOSMock(),
}));

import WindowControls from '../components/WindowControls';

describe('WindowControls', () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('renders windows order on non-mac', () => {
    isMacOSMock.mockReturnValue(false);
    render(<WindowControls />);
    const buttons = screen.getAllByRole('button');
    expect(buttons.map((b) => b.getAttribute('aria-label'))).toEqual([
      '最小化',
      '最大化',
      '关闭',
    ]);
    expect(document.querySelector('.window-controls--windows')).toBeTruthy();
  });

  it('renders mac traffic-light order on mac', () => {
    isMacOSMock.mockReturnValue(true);
    render(<WindowControls />);
    const buttons = screen.getAllByRole('button');
    expect(buttons.map((b) => b.getAttribute('aria-label'))).toEqual([
      '关闭',
      '最小化',
      '最大化',
    ]);
    expect(document.querySelector('.window-controls--mac')).toBeTruthy();
  });
});
