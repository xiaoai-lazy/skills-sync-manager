import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isMacOS } from '../utils/platform';

function WindowsControls() {
  const appWindow = getCurrentWindow();

  return (
    <div className="window-controls window-controls--windows">
      <button
        type="button"
        className="window-btn"
        aria-label="最小化"
        onClick={() => void appWindow.minimize()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M1 5h8" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
      <button
        type="button"
        className="window-btn"
        aria-label="最大化"
        onClick={() => void appWindow.toggleMaximize()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <rect
            x="1.5"
            y="1.5"
            width="7"
            height="7"
            rx="1"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.2"
          />
        </svg>
      </button>
      <button
        type="button"
        className="window-btn window-btn-close"
        aria-label="关闭"
        onClick={() => void appWindow.close()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M2 2l6 6M8 2L2 8" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
    </div>
  );
}

function MacControls() {
  const appWindow = getCurrentWindow();

  return (
    <div className="window-controls window-controls--mac">
      <button
        type="button"
        className="window-btn window-btn-traffic window-btn-close"
        aria-label="关闭"
        onClick={() => void appWindow.close()}
      >
        <svg className="traffic-icon" width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M1.5 1.5l5 5M6.5 1.5l-5 5" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
      <button
        type="button"
        className="window-btn window-btn-traffic window-btn-minimize"
        aria-label="最小化"
        onClick={() => void appWindow.minimize()}
      >
        <svg className="traffic-icon" width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M1.5 4h5" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
      <button
        type="button"
        className="window-btn window-btn-traffic window-btn-maximize"
        aria-label="最大化"
        onClick={() => void appWindow.toggleMaximize()}
      >
        <svg className="traffic-icon" width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M4 1.5v5M1.5 4h5" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
    </div>
  );
}

function WindowControls() {
  if (!isTauri()) {
    return null;
  }

  return isMacOS() ? <MacControls /> : <WindowsControls />;
}

export default WindowControls;
