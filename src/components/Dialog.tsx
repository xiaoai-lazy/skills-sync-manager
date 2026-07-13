import React, { useRef } from 'react';
import { useModalFocus } from '../hooks/useModalFocus';

export interface DialogProps {
  open: boolean;
  title: string;
  children: React.ReactNode;
  actions: React.ReactNode;
  onClose?: () => void;
  closeOnEscape?: boolean;
  closeOnOverlayClick?: boolean;
  descriptionId?: string;
}

function Dialog(props: DialogProps) {
  const {
    open,
    title,
    children,
    actions,
    onClose,
    closeOnEscape = true,
    closeOnOverlayClick = true,
    descriptionId,
  } = props;
  const dialogRef = useRef<HTMLDivElement>(null);
  const titleId = 'dialog-title';

  useModalFocus({
    open,
    containerRef: dialogRef,
    onEscape: onClose,
    escapeEnabled: closeOnEscape,
  });

  if (!open) return null;

  return (
    <div
      className="dialog-overlay"
      role="presentation"
      onClick={onClose && closeOnOverlayClick ? () => onClose() : undefined}
    >
      <div
        ref={dialogRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={descriptionId}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="dialog-header" id={titleId}>
          {title}
        </h2>
        <div className="dialog-body">{children}</div>
        <div className="dialog-actions">{actions}</div>
      </div>
    </div>
  );
}

export default Dialog;
