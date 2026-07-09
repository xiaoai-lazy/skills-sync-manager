import { useEffect, useMemo, useState } from 'react';
import type {
  SkillHubEndpoint,
  SkillRecord,
  SkillRepo,
  SkillWithTargetState,
  Target,
} from '../model/types';
import SourceTree from './skill-hub/SourceTree';
import {
  ALL_NODE_ID,
  matchesInstalledNode,
  nodeTitle,
  resolveSkillRecord,
} from './skill-hub/sourceTreeUtils';
import SkillRow from './SkillRow';
import SkillListEmptyState from './skill-hub/SkillListEmptyState';

export interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  skillRecords: Record<string, SkillRecord>;
  endpoints: SkillHubEndpoint[];
  repos: SkillRepo[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillKey: string, state: import('../model/types').SkillInstallState) => void;
}

function filterSkillsByNode(
  skills: SkillWithTargetState[],
  nodeId: string,
  skillRecords: Record<string, SkillRecord>,
): SkillWithTargetState[] {
  return skills.filter((item) =>
    matchesInstalledNode(
      nodeId,
      item.skill.dirName,
      resolveSkillRecord(item.skill, skillRecords),
      item.skill,
    ),
  );
}

function TargetDetail(props: TargetDetailProps) {
  const { target, skills, skillRecords, endpoints, repos, pendingSkillKey, onToggleSkill } =
    props;
  const [selectedNodeId, setSelectedNodeId] = useState(ALL_NODE_ID);

  useEffect(() => {
    setSelectedNodeId(ALL_NODE_ID);
  }, [target?.id]);

  const filteredSkills = useMemo(
    () => filterSkillsByNode(skills, selectedNodeId, skillRecords),
    [skills, selectedNodeId, skillRecords],
  );

  const listHeader = useMemo(
    () => nodeTitle(selectedNodeId, endpoints, repos),
    [selectedNodeId, endpoints, repos],
  );

  if (!target) {
    return (
      <div className="target-detail empty">
        <h2>未选择目标</h2>
        <p>从侧栏选择一个目标目录，以查看和管理 Skill。</p>
      </div>
    );
  }

  const validSkills = filteredSkills.filter((s) => s.skill.valid);
  const invalidSkills = filteredSkills.filter((s) => !s.skill.valid);
  const installedSkills = skills.map((item) => item.skill);

  return (
    <section className="target-detail">
      <div className="target-hero">
        <h1>{target.name}</h1>
        <div className="target-path" title={target.skillsDir}>
          {target.skillsDir}
        </div>
      </div>

      <div className="hub-split target-split">
        <SourceTree
          tab="installed"
          endpoints={endpoints}
          repos={repos}
          discoverSkills={[]}
          installedSkills={installedSkills}
          skillRecords={skillRecords}
          selectedNodeId={selectedNodeId}
          onSelectNode={setSelectedNodeId}
        />

        <div className="skill-list-panel">
          <div className="skill-list-toolbar">
            <div className="skill-list-toolbar-head">
              <div className="skill-list-head-text">
                <h2 className="skill-list-title">{listHeader.title}</h2>
                <p className="skill-list-sub">{listHeader.sub}</p>
              </div>
            </div>
          </div>

          <div className="target-body target-body-in-panel">
            {validSkills.length === 0 && invalidSkills.length === 0 ? (
              <SkillListEmptyState
                title={skills.length === 0 ? '主库中暂无有效 Skill' : '当前来源下暂无 Skill'}
              />
            ) : (
              <div className="target-list-cards">
                {validSkills.map((item) => (
                  <SkillRow
                    key={item.skill.storageKey}
                    item={item}
                    skillRecords={skillRecords}
                    pending={pendingSkillKey === item.skill.storageKey}
                    onToggle={onToggleSkill}
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
                      key={item.skill.storageKey || item.skill.dirName}
                      item={item}
                      skillRecords={skillRecords}
                      pending={
                        pendingSkillKey ===
                        (item.skill.storageKey || item.skill.dirName)
                      }
                      onToggle={onToggleSkill}
                    />
                  ))}
                </div>
              </section>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

export default TargetDetail;
