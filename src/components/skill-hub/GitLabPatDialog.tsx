import { useEffect, useRef, useState } from 'react';
import { errorMessage } from '../../utils/errorMessage';
import { useModalFocus } from '../../hooks/useModalFocus';

export interface GitLabPatDialogProps {
  open: boolean;
  host: string;
  description: string;
  mode?: 'add' | 'authenticate' | 'update';
  onClose: () => void;
  onSubmit: (pat: string) => Promise<void>;
  submitLabel: string;
}

const PAT_MSG_INVALID = '访问密钥无效或权限不足，请检查后重试';

function GitLabPatDialog(props: GitLabPatDialogProps) {
  const { open, description, mode = 'add', onClose, onSubmit, submitLabel } = props;
  const [pat, setPat] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);

  useModalFocus({
    open,
    containerRef: modalRef,
    initialFocusRef: inputRef,
    onEscape: onClose,
    escapeEnabled: !submitting,
  });

  useEffect(() => {
    if (!open) return;
    setPat('');
    setError(null);
    setSubmitting(false);
  }, [open, description]);

  const handleSubmit = async () => {
    const value = pat.trim();
    if (!value || submitting) return;

    setError(null);
    setSubmitting(true);
    try {
      await onSubmit(value);
      onClose();
    } catch (err) {
      const msg = errorMessage(err);
      setError(msg || PAT_MSG_INVALID);
    } finally {
      setSubmitting(false);
    }
  };

  if (!open) return null;

  const descText =
    mode === 'update' ? (
      <>
        更新 <strong>{description}</strong> 的访问密钥。请输入新的个人访问令牌（PAT）。
      </>
    ) : mode === 'authenticate' ? (
      <>
        为 GitLab 站点 <strong>{description}</strong>{' '}
        配置个人访问令牌（PAT）。同一站点下的来源仓库共用此密钥。
      </>
    ) : (
      <>
        仓库 <strong>{description}</strong> 需要登录后访问。请输入对该站点有读权限的个人访问令牌（PAT）。
      </>
    );

  const canSubmit = pat.trim().length > 0 && !submitting;

  return (
    <div
      className="modal-overlay open credential-pat-overlay"
      role="dialog"
      aria-modal="true"
      aria-labelledby="patModalTitle"
      onClick={() => {
        if (!submitting) onClose();
      }}
    >
      <div ref={modalRef} className="modal" onClick={(e) => e.stopPropagation()}>
        <h3 id="patModalTitle">配置 GitLab 访问密钥</h3>
        <p>{descText}</p>
        <label htmlFor="patInput">访问密钥（PAT）</label>
        <input
          ref={inputRef}
          type="password"
          id="patInput"
          value={pat}
          onChange={(e) => {
            setPat(e.target.value);
            if (error) setError(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && canSubmit) void handleSubmit();
          }}
          placeholder="glpat-xxxxxxxxxxxx"
          autoComplete="off"
        />
        {error && (
          <p className="modal-error show" role="alert">
            {error}
          </p>
        )}
        <div className="modal-actions">
          <button type="button" className="cancel" onClick={onClose} disabled={submitting}>
            取消
          </button>
          <button
            type="button"
            className="btn-primary"
            onClick={() => void handleSubmit()}
            disabled={!canSubmit}
          >
            {submitting ? '验证中…' : submitLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

export default GitLabPatDialog;
