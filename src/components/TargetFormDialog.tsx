import React, { useState, useEffect, useRef } from 'react';
import Dialog from './Dialog';

export interface TargetFormDialogProps {
  open: boolean;
  title: string;
  initialName?: string;
  initialSkillsDir?: string;
  confirmLabel?: string;
  pickDirectoryLabel?: string;
  onPickDirectory?: (currentValue: string) => Promise<string | null>;
  onConfirm: (name: string, skillsDir: string) => void;
  onCancel: () => void;
}

const DIRECTORY_PICKER_ERROR = 'Directory selection failed. Try again or enter the path manually.';

function TargetFormDialog(props: TargetFormDialogProps) {
  const {
    open,
    title,
    initialName = '',
    initialSkillsDir = '',
    confirmLabel = 'Save',
    pickDirectoryLabel = 'Choose Directory',
    onPickDirectory,
    onConfirm,
    onCancel,
  } = props;
  const [name, setName] = useState(initialName);
  const [skillsDir, setSkillsDir] = useState(initialSkillsDir);
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const pickerRequestIdRef = useRef(0);
  const resetGenerationRef = useRef(0);
  const resetKey = `${open}:${initialName}:${initialSkillsDir}`;
  const resetKeyRef = useRef(resetKey);

  if (resetKeyRef.current !== resetKey) {
    resetKeyRef.current = resetKey;
    resetGenerationRef.current += 1;
  }

  useEffect(() => {
    if (open) {
      setName(initialName);
      setSkillsDir(initialSkillsDir);
      setIsPickingDirectory(false);
      setPickerError(null);
    }
  }, [open, initialName, initialSkillsDir]);

  const canSubmit = name.trim().length > 0 && skillsDir.trim().length > 0;

  const handleConfirm = () => {
    if (!canSubmit) return;
    onConfirm(name.trim(), skillsDir.trim());
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && canSubmit) {
      e.preventDefault();
      onConfirm(name.trim(), skillsDir.trim());
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
      const pickedDirectory = await onPickDirectory(skillsDir);
      if (
        pickerRequestIdRef.current !== pickerRequestId ||
        resetGenerationRef.current !== pickerResetGeneration
      ) {
        return;
      }
      if (pickedDirectory !== null) {
        setSkillsDir(pickedDirectory);
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
          <button onClick={handleConfirm} disabled={!canSubmit}>
            {confirmLabel}
          </button>
        </>
      }
    >
      <div className="dialog-form-field">
        <label htmlFor="target-form-name">Target name</label>
        <input
          id="target-form-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={handleKeyDown}
        />
      </div>
      <div className="dialog-form-field">
        <label htmlFor="target-form-skills-dir">Skills directory path</label>
        <div className="directory-input-row">
          <input
            id="target-form-skills-dir"
            type="text"
            value={skillsDir}
            onChange={(e) => setSkillsDir(e.target.value)}
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

export default TargetFormDialog;
