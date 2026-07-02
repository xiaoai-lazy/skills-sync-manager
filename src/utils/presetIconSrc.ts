import { convertFileSrc, isTauri } from '@tauri-apps/api/core';

export function presetIconSrc(iconUrl?: string): string | undefined {
  if (!iconUrl) return undefined;
  if (!isTauri()) return iconUrl;
  return convertFileSrc(iconUrl);
}
