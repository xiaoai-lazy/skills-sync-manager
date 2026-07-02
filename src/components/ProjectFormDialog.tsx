import React, { useEffect, useRef, useState } from 'react';
import Dialog from './Dialog';
import { addProject, updateProject } from '../api/commands';
import { selectDirectory } from '../api/dialog';
import type { AppState, Project } from '../model/types';
import { errorMessage } from '../utils/errorMessage';

export interface ProjectFormDialogProps {
  open: boolean;
  onClose: () => void;
  mode: 'add' | 'edit';
  project?: Project;
  selectedTargetId?: string | null;
  onSuccess: (state: AppState) => void;
}

const DIRECTORY_PICKER_ERROR = '目录选择失败，请重试或手动输入路径。';

function ProjectFormDialog(props: ProjectFormDialogProps) {
  const { open, onClose, mode, project, selectedTargetId, onSuccess } = props;

  const [name, setName] = useState('');
  const [rootPath, setRootPath] = useState('');
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pickerRequestIdRef = useRef(0);
  const resetGenerationRef = useRef(0);
  const resetKey = `${open}:${mode}:${project?.id ?? ''}:${project?.name ?? ''}:${project?.rootPath ?? ''}`;
  const resetKeyRef = useRef(resetKey);

  if (resetKeyRef.current !== resetKey) {
    resetKeyRef.current = resetKey;
    resetGenerationRef.current += 1;
  }

  const isAddMode = mode === 'add';
  const title = isAddMode ? '添加项目' : '编辑项目';
  const confirmLabel = isAddMode ? '添加' : '保存';
  const canSubmit =
    name.trim().length > 0 && (isAddMode ? rootPath.trim().length > 0 : true) && !submitting;

  useEffect(() => {
    if (!open) return;

    setName(project?.name ?? '');
    setRootPath(project?.rootPath ?? '');
    setError(null);
    setPickerError(null);
    setSubmitting(false);
    setIsPickingDirectory(false);
  }, [open, mode, project]);

  const handleSubmit = async () => {
    if (!canSubmit) return;

    setError(null);
    setSubmitting(true);

    try {
      const next = isAddMode
        ? await addProject(name.trim(), rootPath.trim(), selectedTargetId)
        : await updateProject(project!.id, name.trim(), selectedTargetId);
      onSuccess(next);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handlePickDirectory = async () => {
    const pickerRequestId = pickerRequestIdRef.current + 1;
    const pickerResetGeneration = resetGenerationRef.current;
    pickerRequestIdRef.current = pickerRequestId;
    setIsPickingDirectory(true);
    setPickerError(null);

    try {
      const pickedDirectory = await selectDirectory(rootPath);
      if (
        pickerRequestIdRef.current !== pickerRequestId ||
        resetGenerationRef.current !== pickerResetGeneration
      ) {
        return;
      }
      if (pickedDirectory !== null) {
        setRootPath(pickedDirectory);
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && canSubmit) {
      e.preventDefault();
      void handleSubmit();
    }
  };

  return (
    <Dialog
      open={open}
      title={title}
      closeOnEscape={false}
      closeOnOverlayClick={false}
      descriptionId={error ? 'project-form-dialog-error' : undefined}
      actions={
        <>
          <button className="secondary-button" onClick={onClose} disabled={submitting}>
            取消
          </button>
          <button onClick={() => void handleSubmit()} disabled={!canSubmit}>
            {submitting ? `${confirmLabel}中…` : confirmLabel}
          </button>
        </>
      }
    >
      <div className="dialog-form-field">
        <label htmlFor="project-form-name">项目名称</label>
        <input
          id="project-form-name"
          type="text"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
            if (error) setError(null);
          }}
          onKeyDown={handleKeyDown}
          disabled={submitting}
        />
      </div>
      <div className="dialog-form-field">
        <label htmlFor="project-form-root-path">项目根目录</label>
        <div className="directory-input-row">
          <input
            id="project-form-root-path"
            type="text"
            value={rootPath}
            readOnly={!isAddMode}
            onChange={isAddMode ? (e) => setRootPath(e.target.value) : undefined}
            onKeyDown={handleKeyDown}
            disabled={submitting}
          />
          {isAddMode ? (
            <button type="button" onClick={() => void handlePickDirectory()} disabled={submitting || isPickingDirectory}>
              {isPickingDirectory ? '选择中…' : '选择目录'}
            </button>
          ) : null}
        </div>
        {pickerError ? <div className="dialog-field-error">{pickerError}</div> : null}
      </div>

      {error ? (
        <div className="dialog-field-error" id="project-form-dialog-error" role="alert">
          {error}
        </div>
      ) : null}
    </Dialog>
  );
}

export default ProjectFormDialog;
