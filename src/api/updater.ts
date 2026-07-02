import { invoke } from '@tauri-apps/api/core';

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes?: string | null;
}

export async function checkAppUpdate(): Promise<UpdateInfo | null> {
  return invoke<UpdateInfo | null>('check_app_update');
}

export async function installAppUpdate(): Promise<void> {
  return invoke<void>('install_app_update');
}
