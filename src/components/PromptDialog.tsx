import React, { useState, useEffect } from 'react';
import Dialog from './Dialog';

export interface PromptDialogProps {
  open: boolean;
  title: string;
  label: string;
  defaultValue?: string;
  confirmLabel?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

function PromptDialog(props: PromptDialogProps) {
  const { open, title, label, defaultValue = '', confirmLabel = 'OK', onConfirm, onCancel } = props;
  const [value, setValue] = useState(defaultValue);

  useEffect(() => {
    if (open) {
      setValue(defaultValue);
    }
  }, [open, defaultValue]);

  const handleConfirm = () => {
    onConfirm(value);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      onConfirm(value);
    }
  };

  return (
    <Dialog
      open={open}
      title={title}
      onClose={onCancel}
      actions={
        <>
          <button className="secondary-button" onClick={onCancel}>
            Cancel
          </button>
          <button onClick={handleConfirm}>
            {confirmLabel}
          </button>
        </>
      }
    >
      <div className="dialog-form-field">
        <label htmlFor="prompt-dialog-input">{label}</label>
        <input
          id="prompt-dialog-input"
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
        />
      </div>
    </Dialog>
  );
}

export default PromptDialog;
