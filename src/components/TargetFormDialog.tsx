import React, { useState, useEffect } from 'react';
import Dialog from './Dialog';

export interface TargetFormDialogProps {
  open: boolean;
  title: string;
  initialName?: string;
  initialSkillsDir?: string;
  confirmLabel?: string;
  onConfirm: (name: string, skillsDir: string) => void;
  onCancel: () => void;
}

function TargetFormDialog(props: TargetFormDialogProps) {
  const {
    open,
    title,
    initialName = '',
    initialSkillsDir = '',
    confirmLabel = 'Save',
    onConfirm,
    onCancel,
  } = props;
  const [name, setName] = useState(initialName);
  const [skillsDir, setSkillsDir] = useState(initialSkillsDir);

  useEffect(() => {
    if (open) {
      setName(initialName);
      setSkillsDir(initialSkillsDir);
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
        <input
          id="target-form-skills-dir"
          type="text"
          value={skillsDir}
          onChange={(e) => setSkillsDir(e.target.value)}
          onKeyDown={handleKeyDown}
        />
      </div>
    </Dialog>
  );
}

export default TargetFormDialog;
