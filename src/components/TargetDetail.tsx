import type { Target, SkillWithTargetState } from '../model/types';
import SkillRow from './SkillRow';

export interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillDirName: string, state: import('../model/types').SkillInstallState) => void;
}

function TargetDetail(props: TargetDetailProps) {
  if (!props.target) {
    return (
      <div className="target-detail empty">
        <h2>未选择目标</h2>
        <p>从侧栏选择一个目标目录，以查看和管理 Skill。</p>
      </div>
    );
  }

  const validSkills = props.skills.filter((s) => s.skill.valid);
  const invalidSkills = props.skills.filter((s) => !s.skill.valid);

  return (
    <section className="target-detail">
      <div className="target-hero">
        <h1>{props.target.name}</h1>
        <div className="target-path" title={props.target.skillsDir}>
          {props.target.skillsDir}
        </div>
      </div>

      <div className="target-body">
        {validSkills.length === 0 ? (
          <div className="empty-hint">主库中暂无有效 Skill</div>
        ) : (
          <div className="target-list-cards">
            {validSkills.map((item) => (
              <SkillRow
                key={item.skill.dirName}
                item={item}
                pending={props.pendingSkillKey === item.skill.dirName}
                onToggle={props.onToggleSkill}
              />
            ))}
          </div>
        )}

        {invalidSkills.length > 0 && (
          <section className="target-invalid-section">
            <h3 className="target-section-label">无效 Skill（{invalidSkills.length}）</h3>
            <div className="target-list-cards invalid-section">
              {invalidSkills.map((item) => (
                <SkillRow
                  key={item.skill.dirName}
                  item={item}
                  pending={props.pendingSkillKey === item.skill.dirName}
                  onToggle={props.onToggleSkill}
                />
              ))}
            </div>
          </section>
        )}
      </div>
    </section>
  );
}

export default TargetDetail;
