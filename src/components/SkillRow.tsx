import type { SkillWithTargetState, SkillInstallState } from '../model/types';

export interface SkillRowProps {
  item: SkillWithTargetState;
  pending: boolean;
  onToggle: (skillDirName: string, state: SkillInstallState) => void;
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

  const explanation = stateExplanationMap[state];
  const detailMessage =
    state === 'invalidSkill'
      ? skill.validationErrors.join('，')
      : item.message ?? null;

  const displayName = isInvalid
    ? `(无效) ${skill.name ?? skill.dirName}`
    : skill.name ?? skill.dirName;
  const desc = skill.description ?? skill.dirName;

  return (
    <div
      className={`target-skill-card ${isInvalid ? 'invalid' : ''} ${pending ? 'pending' : ''}`}
    >
      <div className="skill-info">
        <div className="skill-name">{displayName}</div>
        <div className="skill-desc">{desc}</div>
        {(explanation || detailMessage) && (
          <div className="skill-message">
            {explanation && detailMessage
              ? `${explanation}：${detailMessage}`
              : (explanation || detailMessage)}
          </div>
        )}
      </div>
      <span className={`status-badge status-${state}`}>{stateLabel(state)}</span>
      <div className="skill-actions">
        <input
          type="checkbox"
          checked={isInstalled}
          disabled={!toggleEnabled}
          onChange={() => props.onToggle(skill.dirName, state)}
          title={toggleEnabled ? (isInstalled ? '卸载' : '安装') : '无法切换'}
          aria-label={`${displayName} 安装状态`}
        />
      </div>
    </div>
  );
}

export default SkillRow;
