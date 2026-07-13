/** Detect macOS for platform-adaptive chrome (traffic lights on the left). */
export function isMacOS(): boolean {
  if (typeof navigator === 'undefined') return false;

  const platform =
    // Chromium userAgentData when available
    (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData
      ?.platform ??
    navigator.platform ??
    '';

  if (/mac/i.test(platform)) return true;
  return /Mac|iPhone|iPod|iPad/.test(navigator.userAgent);
}
