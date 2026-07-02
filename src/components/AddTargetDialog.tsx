import React, { useEffect, useMemo, useRef, useState } from 'react';
import Dialog from './Dialog';
import { addAgentTarget, addCustomTarget, listAgentPresets } from '../api/commands';
import { selectDirectory } from '../api/dialog';
import type { AgentPreset, AppState, Target, TargetScope } from '../model/types';
import { errorMessage } from '../utils/errorMessage';
import { presetIconSrc } from '../utils/presetIconSrc';

export interface AddTargetDialogProps {
  open: boolean;
  onClose: () => void;
  scope: TargetScope;
  projectId?: string;
  projectName?: string;
  existingTargets: Target[];
  selectedTargetId?: string | null;
  onSuccess: (state: AppState) => void;
}

const DIRECTORY_PICKER_ERROR = '目录选择失败，请重试或手动输入路径。';

function presetPath(preset: AgentPreset, scope: TargetScope): string {
  if (scope === 'project' && preset.projectRelativePath) {
    return preset.projectRelativePath;
  }
  return preset.globalPath;
}

function presetInitial(name: string): string {
  const trimmed = name.trim();
  return trimmed ? trimmed.charAt(0).toUpperCase() : '?';
}

function AddTargetDialog(props: AddTargetDialogProps) {
  const {
    open,
    onClose,
    scope,
    projectId,
    projectName,
    existingTargets,
    selectedTargetId,
    onSuccess,
  } = props;

  const [presets, setPresets] = useState<AgentPreset[]>([]);
  const [loadingPresets, setLoadingPresets] = useState(false);
  const [name, setName] = useState('');
  const [skillsDir, setSkillsDir] = useState('');
  const [isPickingDirectory, setIsPickingDirectory] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pickerRequestIdRef = useRef(0);
  const resetGenerationRef = useRef(0);
  const resetKey = `${open}:${scope}:${projectId ?? ''}`;
  const resetKeyRef = useRef(resetKey);

  if (resetKeyRef.current !== resetKey) {
    resetKeyRef.current = resetKey;
    resetGenerationRef.current += 1;
  }

  const title =
    scope === 'global'
      ? '添加目标（用户级）'
      : `添加目标 · ${projectName?.trim() || '项目'}`;

  const existingAgentIds = useMemo(() => {
    return new Set(
      existingTargets
        .filter(
          (target) =>
            target.scope === scope &&
            target.kind === 'agent' &&
            target.agentId &&
            (scope === 'global' || target.projectId === projectId),
        )
        .map((target) => target.agentId as string),
    );
  }, [existingTargets, scope, projectId]);

  const availablePresets = useMemo(
    () => presets.filter((preset) => !existingAgentIds.has(preset.id)),
    [presets, existingAgentIds],
  );

  const showQuickAdd = availablePresets.length > 0;
  const canSubmit = name.trim().length > 0 && skillsDir.trim().length > 0 && !submitting;

  useEffect(() => {
    if (!open) return;

    setName('');
    setSkillsDir('');
    setError(null);
    setPickerError(null);
    setSubmitting(false);
    setPresets([]);
    setLoadingPresets(true);

    let cancelled = false;

    void listAgentPresets(scope, projectId)
      .then((list) => {
        if (!cancelled) {
          setPresets(list);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(errorMessage(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingPresets(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [open, scope, projectId]);

  const handleAddPreset = async (agentId: string) => {
    if (submitting) return;

    setError(null);
    setSubmitting(true);

    try {
      const next = await addAgentTarget(scope, agentId, projectId, selectedTargetId ?? null);
      onSuccess(next);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleSubmit = async () => {
    if (!canSubmit) return;

    setError(null);
    setSubmitting(true);

    try {
      const next = await addCustomTarget(
        scope,
        name.trim(),
        skillsDir.trim(),
        projectId,
        selectedTargetId ?? null,
      );
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
      const pickedDirectory = await selectDirectory(skillsDir);
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
      descriptionId={error ? 'add-target-dialog-error' : undefined}
      actions={
        <>
          <button className="secondary-button" onClick={onClose} disabled={submitting}>
            取消
          </button>
          <button onClick={() => void handleSubmit()} disabled={!canSubmit}>
            {submitting ? '添加中…' : '添加'}
          </button>
        </>
      }
    >
      {showQuickAdd ? (
        <>
          <div className="quick-add-section">
            <div className="quick-add-label">快捷添加</div>
            <div className="quick-add-row">
              {availablePresets.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  className="quick-add-chip"
                  title={presetPath(preset, scope)}
                  disabled={submitting}
                  onClick={() => void handleAddPreset(preset.id)}
                >
                  {preset.iconUrl ? (
                    <img
                      className="quick-add-chip-icon"
                      src={presetIconSrc(preset.iconUrl)}
                      alt=""
                    />
                  ) : (
                    <span className="quick-add-chip-fallback" aria-hidden="true">
                      {presetInitial(preset.displayName)}
                    </span>
                  )}
                  <span className="quick-add-chip-label">{preset.displayName}</span>
                </button>
              ))}
            </div>
          </div>
          <div className="quick-add-divider" role="separator">
            <span>或</span>
          </div>
        </>
      ) : loadingPresets ? (
        <div className="dialog-loading-hint">加载预设中…</div>
      ) : null}

      <div className="dialog-form-field">
        <label htmlFor="add-target-name">目标名称</label>
        <input
          id="add-target-name"
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
        <label htmlFor="add-target-skills-dir">Skill 目录路径</label>
        <div className="directory-input-row">
          <input
            id="add-target-skills-dir"
            type="text"
            value={skillsDir}
            readOnly
            onKeyDown={handleKeyDown}
            disabled={submitting}
          />
          <button type="button" onClick={() => void handlePickDirectory()} disabled={submitting || isPickingDirectory}>
            {isPickingDirectory ? '选择中…' : '选择目录'}
          </button>
        </div>
        {pickerError ? <div className="dialog-field-error">{pickerError}</div> : null}
      </div>

      {error ? (
        <div className="dialog-field-error" id="add-target-dialog-error" role="alert">
          {error}
        </div>
      ) : null}
    </Dialog>
  );
}

export default AddTargetDialog;
