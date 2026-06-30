import React, { useEffect } from 'react';

export interface KeysManageDialogProps {
  open: boolean;
  hosts: string[];
  onClose: () => void;
  onUpdate: (host: string) => void;
  onRemove: (host: string) => Promise<void>;
}

function KeysManageDialog(props: KeysManageDialogProps) {
  const { open, hosts, onClose, onUpdate, onRemove } = props;

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="modal-overlay open"
      role="dialog"
      aria-modal="true"
      aria-labelledby="keysModalTitle"
      onClick={onClose}
    >
      <div className="modal modal-wide" onClick={(e) => e.stopPropagation()}>
        <h3 id="keysModalTitle">密钥管理</h3>
        <p className="keys-hint">
          用于访问私有 GitLab 站点，按域名保存。同一站点下的来源仓库共用密钥。
        </p>
        {hosts.length === 0 ? (
          <p className="keys-empty">暂无已保存的 GitLab 访问密钥</p>
        ) : (
          hosts.map((host) => (
            <div key={host} className="key-item">
              <div className="key-item-left">
                <span>GitLab · {host}</span>
                <span className="key-status">已配置</span>
              </div>
              <div className="key-actions">
                <button type="button" onClick={() => onUpdate(host)}>
                  更新
                </button>
                <button type="button" onClick={() => void onRemove(host)}>
                  移除
                </button>
              </div>
            </div>
          ))
        )}
        <div className="modal-actions">
          <button type="button" className="cancel" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}

export default KeysManageDialog;
