import type { SkillWithTargetState, SkillInstallState, SkillRecord } from '../model/types';
import { skillSourceLabelForView } from '../utils/skillSourceLabel';

export interface SkillRowProps {
  item: SkillWithTargetState;
  skillRecords?: Record<string, SkillRecord>;
  pending: boolean;
  onToggle: (skillKey: string, state: SkillInstallState) => void;
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
  mismatch: '链接与记录不符，请到目标目录手动删除该链接后重启应用',
  missing: '安装记录存在，但链接已不存在（重启应用将自动恢复）',
  sourceMissing: '源 skill 已不存在（启动时会自动清理安装记录）',
  invalidSkill: null,
};

export function canToggle(state: SkillInstallState): boolean {
  return state === 'notInstalled' || state === 'installed';
}

export function stateLabel(state: SkillInstallState): string {
  return statusLabelMap[state];
}

function SkillRow(props: SkillRowProps) {
  const { item, skillRecords, pending } = props;
  const { skill, state } = item;

  const isInvalid = !skill.valid || state === 'invalidSkill';
  const toggleEnabled = canToggle(state) && !pending;
  const isInstalled = state === 'installed';

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
    : isInstalled
      ? '卸载'
      : '安装';

  return (
    <div
      className={`target-skill-card ${isInvalid ? 'invalid' : ''} ${pending ? 'pending' : ''}`}
    >
      <div className="skill-info">
        <div className="skill-name">{displayName}</div>
        <div className="skill-desc">{desc}</div>
        <div className="skill-source-meta">{sourceMeta}</div>
        {showMessage && <div className="skill-message">{messageText}</div>}
      </div>
      <span className={`status-badge status-${state}`}>{stateLabel(state)}</span>
      <div className="skill-actions">
        <input
          type="checkbox"
          checked={isInstalled}
          disabled={!toggleEnabled}
          onChange={() => props.onToggle(skillKey, state)}
          title={toggleTitle}
          aria-label={`${displayName} 安装状态`}
        />
      </div>
    </div>
  );
}

export default SkillRow;
