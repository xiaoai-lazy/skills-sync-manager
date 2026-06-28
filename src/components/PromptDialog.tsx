import React, { useState, useEffect, useRef } from 'react';
import Dialog from './Dialog';

export interface PromptDialogProps {
  open: boolean;
  title: string;
  label: string;
  defaultValue?: string;
  confirmLabel?: string;
  pickDirectoryLabel?: string;
  onPickDirectory?: (currentValue: string) => Promise<string | null>;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

const DIRECTORY_PICKER_ERROR = 'Directory selection failed. Try again or enter the path manually.';

function PromptDialog(props: PromptDialogProps) {
  const {
    open,
    title,
    label,
    defaultValue = '',
    confirmLabel = 'OK',
    pickDirectoryLabel = 'Choose Directory',
    onPickDirectory,
    onConfirm,
    onCancel,
  } = props;
  const [value, setValue] = useState(defaultValue);
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const pickerRequestIdRef = useRef(0);
  const resetGenerationRef = useRef(0);
  const resetKey = `${open}:${defaultValue}`;
  const resetKeyRef = useRef(resetKey);

  if (resetKeyRef.current !== resetKey) {
    resetKeyRef.current = resetKey;
    resetGenerationRef.current += 1;
  }

  useEffect(() => {
    if (open) {
      setValue(defaultValue);
      setIsPickingDirectory(false);
      setPickerError(null);
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

  const handlePickDirectory = async () => {
    if (!onPickDirectory) {
      return;
    }

    const pickerRequestId = pickerRequestIdRef.current + 1;
    const pickerResetGeneration = resetGenerationRef.current;
    pickerRequestIdRef.current = pickerRequestId;
    setIsPickingDirectory(true);
    setPickerError(null);

    try {
      const pickedDirectory = await onPickDirectory(value);
      if (
        pickerRequestIdRef.current !== pickerRequestId ||
        resetGenerationRef.current !== pickerResetGeneration
      ) {
        return;
      }
      if (pickedDirectory !== null) {
        setValue(pickedDirectory);
      }
    } catch {
      if (
        pickerRequestIdRef.current === pickerRequestId &&
        resetGenerationRef.current === pickerResetGeneration
      ) {
        setPickerError(DIRECTORY_PICKER_ERROR);
      }
    } finally {
      if (
        pickerRequestIdRef.current === pickerRequestId &&
        resetGenerationRef.current === pickerResetGeneration
      ) {
        setIsPickingDirectory(false);
      }
    }
  };

  return (
    <Dialog
      open={open}
      title={title}
      closeOnEscape={false}
      closeOnOverlayClick={false}
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
        <div className="directory-input-row">
          <input
            id="prompt-dialog-input"
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          {onPickDirectory ? (
            <button type="button" onClick={handlePickDirectory} disabled={isPickingDirectory}>
              {isPickingDirectory ? 'Choosing...' : pickDirectoryLabel}
            </button>
          ) : null}
        </div>
        {pickerError ? <div className="dialog-field-error">{pickerError}</div> : null}
      </div>
    </Dialog>
  );
}

export default PromptDialog;
