import { open } from '@tauri-apps/plugin-dialog';

export async function selectDirectory(defaultPath?: string): Promise<string | null> {
  const trimmedDefaultPath = defaultPath?.trim();
  const selected = await open({
    directory: true,
    multiple: false,
    ...(trimmedDefaultPath ? { defaultPath: trimmedDefaultPath } : {}),
  });

  return typeof selected === 'string' ? selected : null;
}
