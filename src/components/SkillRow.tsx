import React from 'react';
import type { SkillWithTargetState, SkillInstallState } from '../model/types';

export interface SkillRowProps {
  item: SkillWithTargetState;
  pending: boolean;
  onToggle: (skillDirName: string, state: SkillInstallState) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
}

const statusLabelMap: Record<SkillInstallState, string> = {
  notInstalled: '未安装',
  installed: '已安装',
  conflict: '冲突',
  mismatch: '异常',
  missing: '缺失',
  sourceMissing: '源缺失',
  invalidSkill: '无效 skill',
};

const stateExplanationMap: Record<SkillInstallState, string | null> = {
  notInstalled: null,
  installed: null,
  conflict: '目标路径已存在同名内容',
  mismatch: '链接目标与记录不符',
  missing: '安装记录存在，但链接已不存在',
  sourceMissing: '源 skill 已不存在',
  invalidSkill: null,
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

  const explanation = stateExplanationMap[state];
  const detailMessage =
    state === 'invalidSkill'
      ? skill.validationErrors.join(', ')
      : item.message ?? null;

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
        {(explanation || detailMessage) && (
          <div className="skill-message">
            {explanation && detailMessage
              ? `${explanation}：${detailMessage}`
              : (explanation || detailMessage)}
          </div>
        )}
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
