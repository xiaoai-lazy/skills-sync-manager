import React from 'react';
import type { Target, SkillWithTargetState } from '../model/types';
import SkillRow from './SkillRow';

export interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillDirName: string, state: import('../model/types').SkillInstallState) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
}

function TargetDetail(props: TargetDetailProps) {
  if (!props.target) {
    return (
      <div className="target-detail empty">
        <h2>No Target Selected</h2>
        <p>Select a target from the sidebar to view and manage its skills.</p>
      </div>
    );
  }

  const validSkills = props.skills.filter((s) => s.skill.valid);
  const invalidSkills = props.skills.filter((s) => !s.skill.valid);

  return (
    <div className="target-detail">
      <header className="target-detail-header">
        <h2>{props.target.name}</h2>
        <div className="target-meta" title={props.target.skillsDir}>
          {props.target.skillsDir}
        </div>
      </header>

      <section className="skill-section">
        <h3>Skills ({validSkills.length})</h3>
        {validSkills.length === 0 ? (
          <div className="empty-state">
            <p>No valid skills found in the main library.</p>
          </div>
        ) : (
          <ul className="skill-list">
            {validSkills.map((item) => (
              <li key={item.skill.dirName}>
                <SkillRow
                  item={item}
                  pending={props.pendingSkillKey === item.skill.dirName}
                  onToggle={props.onToggleSkill}
                  onDeleteMainSkill={props.onDeleteMainSkill}
                />
              </li>
            ))}
          </ul>
        )}
      </section>

      {invalidSkills.length > 0 && (
        <section className="skill-section invalid-section">
          <h3>Invalid Skills ({invalidSkills.length})</h3>
          <ul className="skill-list">
            {invalidSkills.map((item) => (
              <li key={item.skill.dirName}>
                <SkillRow
                  item={item}
                  pending={props.pendingSkillKey === item.skill.dirName}
                  onToggle={props.onToggleSkill}
                  onDeleteMainSkill={props.onDeleteMainSkill}
                />
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export default TargetDetail;
