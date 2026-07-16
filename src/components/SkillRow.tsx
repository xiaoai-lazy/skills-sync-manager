import type { KeyboardEvent, MouseEvent } from 'react';
import type { SkillWithTargetState, SkillInstallState, SkillRecord } from '../model/types';
import { skillSourceLabelForView } from '../utils/skillSourceLabel';

export interface SkillRowProps {
  item: SkillWithTargetState;
  skillRecords?: Record<string, SkillRecord>;
  pending: boolean;
  onToggle: (skillKey: string, state: SkillInstallState) => void;
  onPreview?: (storageKey: string) => void;
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
  mismatch: '链接与记录不符；可强制清除安装记录，异常路径需手动删除',
  missing: '安装记录存在但链接已缺失；可强制清除安装记录',
  sourceMissing: '源 skill 已不存在；可强制清除安装记录',
  invalidSkill: null,
};

export function canForceClearInstallation(state: SkillInstallState): boolean {
  return state === 'mismatch' || state === 'missing' || state === 'sourceMissing';
}

export function canToggle(state: SkillInstallState): boolean {
  return state === 'notInstalled' || state === 'installed' || canForceClearInstallation(state);
}

export function stateLabel(state: SkillInstallState): string {
  return statusLabelMap[state];
}

function SkillRow(props: SkillRowProps) {
  const { item, skillRecords, pending, onPreview, onToggle } = props;
  const { skill, state } = item;

  const isInvalid = !skill.valid || state === 'invalidSkill';
  const toggleEnabled = canToggle(state) && !pending;
  const isInstalled = state === 'installed' || canForceClearInstallation(state);

  const explanation = stateExplanationMap[state];
  const detailMessage =
    state === 'invalidSkill'
      ? skill.validationErrors.join('，')
      : item.message ?? null;
  const showMessage =
    state !== 'notInstalled' &&
    state !== 'installed' &&
    Boolean(explanation || detailMessage);
  const messageText = (() => {
    if (!showMessage) return null;
    if (explanation && detailMessage) {
      if (detailMessage.includes('手动删除') || detailMessage.includes(explanation)) {
        return detailMessage;
      }
      return `${explanation}：${detailMessage}`;
    }
    return explanation || detailMessage;
  })();

  const displayName = isInvalid
    ? `(无效) ${skill.name ?? skill.dirName}`
    : skill.name ?? skill.dirName;
  const desc = skill.description ?? skill.dirName;
  const sourceMeta = skillSourceLabelForView(skill, skillRecords);
  const skillKey = skill.storageKey;
  const toggleTitle = !toggleEnabled
    ? '无法切换'
    : state === 'installed'
      ? '点击卸载'
      : canForceClearInstallation(state)
        ? '点击强制清除安装记录'
        : '点击安装';

  const handleToggle = () => {
    if (!toggleEnabled) return;
    onToggle(skillKey, state);
  };

  const handleCardClick = () => {
    handleToggle();
  };

  const handleCardKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!toggleEnabled) return;
    if (event.key === ' ' || event.key === 'Enter') {
      event.preventDefault();
      handleToggle();
    }
  };

  const handlePreviewClick = (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    onPreview?.(skillKey);
  };

  const className = [
    'target-skill-card',
    isInstalled ? 'installed' : '',
    toggleEnabled ? 'toggleable' : '',
    isInvalid ? 'invalid' : '',
    pending ? 'pending' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      className={className}
      role="checkbox"
      aria-checked={isInstalled}
      aria-disabled={!toggleEnabled}
      aria-label={`${displayName} 安装状态`}
      title={toggleTitle}
      tabIndex={toggleEnabled ? 0 : -1}
      onClick={handleCardClick}
      onKeyDown={handleCardKeyDown}
    >
      <div className="skill-info">
        <button
          type="button"
          className="skill-name skill-name-link"
          onClick={handlePreviewClick}
        >
          {displayName}
        </button>
        <div className="skill-desc">{desc}</div>
        <div className="skill-source-meta">{sourceMeta}</div>
        {showMessage && <div className="skill-message">{messageText}</div>}
      </div>
      <span className={`status-badge status-${state}`}>{stateLabel(state)}</span>
    </div>
  );
}

export default SkillRow;
