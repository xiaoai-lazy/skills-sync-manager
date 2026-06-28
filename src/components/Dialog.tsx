import React, { useEffect, useRef } from 'react';

export interface DialogProps {
  open: boolean;
  title: string;
  children: React.ReactNode;
  actions: React.ReactNode;
  onClose?: () => void;
  descriptionId?: string;
}

function Dialog(props: DialogProps) {
  const { open, title, children, actions, onClose, descriptionId } = props;
  const dialogRef = useRef<HTMLDivElement>(null);
  const titleId = 'dialog-title';

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && onClose) {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown);

    const focusable = dialogRef.current?.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    focusable?.focus();

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="dialog-overlay"
      role="presentation"
      onClick={onClose}
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
