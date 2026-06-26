import React from 'react';
import type { SkillWithTargetState, SkillInstallState } from '../model/types';

export interface SkillRowProps {
  item: SkillWithTargetState;
  pending: boolean;
  onToggle: (skillDirName: string, state: SkillInstallState) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
}

const statusLabelMap: Record<SkillInstallState, string> = {
  notInstalled: 'Not Installed',
  installed: 'Installed',
  conflict: 'Conflict',
  mismatch: 'Mismatch',
  missing: 'Missing',
  sourceMissing: 'Source Missing',
  invalidSkill: 'Invalid',
};

function SkillRow(props: SkillRowProps) {
  const { item, pending } = props;
  const { skill, state } = item;

  const isInvalid = !skill.valid || state === 'invalidSkill';
  const canToggle = !isInvalid && !pending;
  const isInstalled = state === 'installed';

  return (
    <div className={`skill-row ${isInvalid ? 'invalid' : ''} ${pending ? 'pending' : ''}`}>
      <div className="skill-info">
        <div className="skill-name">{skill.name ?? skill.dirName}</div>
        {skill.description && (
          <div className="skill-description">{skill.description}</div>
        )}
        <div className="skill-dir">{skill.dirName}</div>
      </div>
      <div className="skill-status">
        <span className={`status-badge status-${state}`}>
          {statusLabelMap[state]}
        </span>
      </div>
      <div className="skill-actions">
        <button
          className="toggle-button"
          disabled={!canToggle}
          onClick={() => props.onToggle(skill.dirName, state)}
          title={canToggle ? (isInstalled ? 'Uninstall' : 'Install') : 'Cannot toggle'}
        >
          {isInstalled ? 'Uninstall' : 'Install'}
        </button>
        <button
          className="danger-button"
          disabled={pending}
          onClick={() => props.onDeleteMainSkill(skill.dirName)}
          title="Delete from main library"
        >
          Delete
        </button>
      </div>
    </div>
  );
}

export default SkillRow;
