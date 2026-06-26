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

export function canToggle(state: SkillInstallState): boolean {
  return state === 'notInstalled' || state === 'installed';
}

export function stateLabel(state: SkillInstallState): string {
  return statusLabelMap[state];
}

function SkillRow(props: SkillRowProps) {
  const { item, pending } = props;
  const { skill, state } = item;

  const isInvalid = !skill.valid || state === 'invalidSkill';
  const toggleEnabled = canToggle(state) && !pending;
  const isInstalled = state === 'installed';
  const canDelete = skill.valid && state !== 'invalidSkill' && !pending;

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
        <input
          type="checkbox"
          checked={isInstalled}
          disabled={!toggleEnabled}
          onChange={() => props.onToggle(skill.dirName, state)}
          title={toggleEnabled ? (isInstalled ? 'Uninstall' : 'Install') : 'Cannot toggle'}
        />
        <button
          className="danger-button"
          disabled={!canDelete}
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
