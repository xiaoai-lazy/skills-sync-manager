import Dialog from './Dialog';
import type { UpdateInfo } from '../api/updater';

export interface UpdateDialogProps {
  open: boolean;
  update: UpdateInfo | null;
  installing: boolean;
  error: string | null;
  onDefer: () => void;
  onInstall: () => void;
}

function UpdateDialog(props: UpdateDialogProps) {
  const { open, update, installing, error, onDefer, onInstall } = props;

  if (!update) return null;

  return (
    <Dialog
      open={open}
      title="发现新版本"
      closeOnEscape={false}
      closeOnOverlayClick={false}
      actions={
        <>
          <button
            type="button"
            className="secondary-button"
            onClick={onDefer}
            disabled={installing}
          >
            稍后
          </button>
          <button
            type="button"
            onClick={onInstall}
            disabled={installing}
          >
            {installing ? '正在更新…' : '立即更新'}
          </button>
        </>
      }
    >
      <p className="update-dialog-version">
        当前版本 <strong>{update.currentVersion}</strong> → 新版本{' '}
        <strong>{update.version}</strong>
      </p>
      {update.notes ? <p className="update-dialog-notes">{update.notes}</p> : null}
      {error ? <p className="update-dialog-error">{error}</p> : null}
    </Dialog>
  );
}

export default UpdateDialog;
