import type { AppErrorDto } from '../model/types';

export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;

  if (err && typeof err === 'object') {
    const dto = err as Partial<AppErrorDto>;
    if (typeof dto.message === 'string' && dto.message.length > 0) {
      return dto.message;
    }
  }

  return '操作失败，请查看日志或重试。';
}
