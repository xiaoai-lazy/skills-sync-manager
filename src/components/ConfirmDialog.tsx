import { useEffect, useRef } from 'react';
import Dialog from './Dialog';

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function ConfirmDialog(props: ConfirmDialogProps) {
  const { open, title, message, confirmLabel, cancelLabel, danger, onConfirm, onCancel } = props;
  const confirmRef = useRef<HTMLButtonElement>(null);
  const messageId = 'confirm-dialog-message';

  useEffect(() => {
    if (open) {
      confirmRef.current?.focus();
    }
  }, [open]);

  return (
    <Dialog
      open={open}
      title={title}
      descriptionId={messageId}
      onClose={onCancel}
      actions={
        <>
          <button className="secondary-button" onClick={onCancel}>
            {cancelLabel}
          </button>
          <button
            ref={confirmRef}
            className={danger ? 'danger-button' : ''}
            onClick={onConfirm}
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
