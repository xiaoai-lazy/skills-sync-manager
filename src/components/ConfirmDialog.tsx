import { useEffect, useRef } from 'react';
import Dialog from './Dialog';

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  /** When true, confirm and cancel are disabled and handlers are ignored. */
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function ConfirmDialog(props: ConfirmDialogProps) {
  const {
    open,
    title,
    message,
    confirmLabel,
    cancelLabel,
    danger,
    busy = false,
    onConfirm,
    onCancel,
  } = props;
  const confirmRef = useRef<HTMLButtonElement>(null);
  /** Sync guard so a double-click before re-render cannot fire twice. */
  const busyGuardRef = useRef(false);
  const messageId = 'confirm-dialog-message';

  useEffect(() => {
    if (open) {
      confirmRef.current?.focus();
    }
  }, [open]);

  // Reset sync guard when the dialog closes, busy clears, or the confirm
  // action identity changes (e.g. delete → force-delete on the same instance).
  useEffect(() => {
    busyGuardRef.current = false;
  }, [open, busy, title, confirmLabel]);

  const handleConfirm = () => {
    if (busy || busyGuardRef.current) return;
    busyGuardRef.current = true;
    onConfirm();
  };

  const handleCancel = () => {
    if (busy || busyGuardRef.current) return;
    onCancel();
  };

  return (
    <Dialog
      open={open}
      title={title}
      descriptionId={messageId}
      onClose={handleCancel}
      closeOnEscape={!busy}
      closeOnOverlayClick={!busy}
      actions={
        <>
          <button
            type="button"
            className="secondary-button"
            onClick={handleCancel}
            disabled={busy}
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            ref={confirmRef}
            className={danger ? 'danger-button' : ''}
            onClick={handleConfirm}
            disabled={busy}
          >
            {confirmLabel}
          </button>
        </>
      }
    >
      <div id={messageId}>{message}</div>
    </Dialog>
  );
}

export default ConfirmDialog;
