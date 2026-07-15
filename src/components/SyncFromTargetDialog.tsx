import React, { useEffect, useState } from 'react';
import Dialog from './Dialog';
import type { SyncTargetInstallationsResponse, Target } from '../model/types';
import {
  defaultSyncSourceId,
  type SyncSourceCandidate,
} from '../utils/targetSyncCandidates';
import { errorMessage } from '../utils/errorMessage';

export interface SyncFromTargetDialogProps {
  open: boolean;
  mode: 'post-create' | 'manual';
  destTarget: Target;
  candidates: SyncSourceCandidate[];
  onClose: () => void;
  onConfirm: (sourceTargetId: string) => Promise<SyncTargetInstallationsResponse>;
}

function isAllFailed(summary: SyncTargetInstallationsResponse): boolean {
  return summary.installed === 0 && summary.failed.length > 0 && summary.skipped === 0;
}

function SyncFromTargetDialog(props: SyncFromTargetDialogProps) {
  const { open, mode, destTarget, candidates, onClose, onConfirm } = props;
  const [sourceId, setSourceId] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<SyncTargetInstallationsResponse | null>(null);

  // Reset only when the dialog opens. Do not depend on `candidates` — parent may
  // pass a new array reference after AppState refresh while failures are shown.
  useEffect(() => {
    if (!open) return;
    setSourceId(defaultSyncSourceId(candidates) ?? '');
    setSubmitting(false);
    setError(null);
    setSummary(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- intentionally open-only
  }, [open]);

  const title =
    mode === 'post-create' ? `已添加 ${destTarget.name}` : '从项目内其他目录同步';
  const secondaryLabel = mode === 'post-create' ? '暂时跳过' : '取消';
  const showSyncButton = !summary || isAllFailed(summary);
  const closeLabel =
    summary && !isAllFailed(summary) ? '关闭' : secondaryLabel;

  const handleSync = async () => {
    if (!sourceId || submitting) return;

    setSubmitting(true);
    setError(null);

    try {
      const response = await onConfirm(sourceId);
      setSummary(response);

      if (response.failed.length === 0) {
        onClose();
        return;
      }

      if (isAllFailed(response)) {
        setError('同步未能安装任何 Skill，请查看失败详情。');
      }
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog
      open={open}
      title={title}
      closeOnEscape={!submitting}
      closeOnOverlayClick={!submitting}
      onClose={submitting ? undefined : onClose}
      descriptionId={error ? 'sync-from-target-error' : undefined}
      actions={
        <>
          <button className="secondary-button" onClick={onClose} disabled={submitting}>
            {closeLabel}
          </button>
          {showSyncButton ? (
            <button onClick={() => void handleSync()} disabled={!sourceId || submitting}>
              {submitting ? '同步中…' : '同步安装'}
            </button>
          ) : null}
        </>
      }
    >
      {mode === 'post-create' ? (
        <p className="dialog-lead">
          可以从项目内其他目录同步已安装的 Skill（以链接方式安装，不会复制文件）。
        </p>
      ) : (
        <p className="dialog-lead">将把源目录已安装的 Skill 安装到当前目录；已存在的会跳过。</p>
      )}

      <div className="dialog-form-field">
        <label htmlFor="sync-source-target">源目录</label>
        <select
          id="sync-source-target"
          value={sourceId}
          disabled={submitting || Boolean(summary && summary.failed.length === 0)}
          onChange={(e) => setSourceId(e.target.value)}
        >
          {candidates.map((c) => (
            <option key={c.target.id} value={c.target.id}>
              {c.target.name} · {c.target.skillsDir} · 已装 {c.installedCount}
            </option>
          ))}
        </select>
      </div>

      {summary ? (
        <div className="dialog-sync-summary" role="status">
          已安装 {summary.installed}，跳过 {summary.skipped}，失败 {summary.failed.length}
          {summary.failed.length > 0 ? (
            <ul>
              {summary.failed.map((f) => (
                <li key={f.storageKey}>
                  {f.label}: {f.error}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      {error ? (
        <div className="dialog-field-error" id="sync-from-target-error" role="alert">
          {error}
        </div>
      ) : null}
    </Dialog>
  );
}

export default SyncFromTargetDialog;
