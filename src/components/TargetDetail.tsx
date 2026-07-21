import { useCallback, useEffect, useMemo, useState } from 'react';
import type {
  IflytekSkillHubEndpoint,
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
import {
  compareSkillDisplayName,
  countTargetNodeSkills,
  formatNodeCount,
  sortTargetSkillRows,
} from '../utils/targetSkillList';

export interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  skillRecords: Record<string, SkillRecord>;
  endpoints: SkillHubEndpoint[];
  iflytekEndpoints?: IflytekSkillHubEndpoint[];
  repos: SkillRepo[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillKey: string, state: import('../model/types').SkillInstallState) => void;
  onPreviewSkill?: (storageKey: string) => void;
  showSyncButton?: boolean;
  onOpenSync?: () => void;
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
  const {
    target,
    skills,
    skillRecords,
    endpoints,
    iflytekEndpoints = [],
    repos,
    pendingSkillKey,
    onToggleSkill,
    onPreviewSkill,
    showSyncButton,
    onOpenSync,
  } = props;
  const [selectedNodeId, setSelectedNodeId] = useState(ALL_NODE_ID);

  useEffect(() => {
    setSelectedNodeId(ALL_NODE_ID);
  }, [target?.id]);

  const filteredSkills = useMemo(
    () => filterSkillsByNode(skills, selectedNodeId, skillRecords),
    [skills, selectedNodeId, skillRecords],
  );

  const listHeader = useMemo(
    () => nodeTitle(selectedNodeId, endpoints, repos, iflytekEndpoints),
    [selectedNodeId, endpoints, repos, iflytekEndpoints],
  );

  const validSkills = useMemo(
    () => filteredSkills.filter((s) => s.skill.valid),
    [filteredSkills],
  );
  const invalidSkills = useMemo(
    () => filteredSkills.filter((s) => !s.skill.valid),
    [filteredSkills],
  );
  const sortedValid = useMemo(
    () => sortTargetSkillRows(validSkills),
    [validSkills],
  );
  const sortedInvalid = useMemo(
    () =>
      [...invalidSkills].sort((a, b) =>
        compareSkillDisplayName(a.skill, b.skill),
      ),
    [invalidSkills],
  );
  const nodeCountLabel = useCallback(
    (nodeId: string) => {
      const { installed, total } = countTargetNodeSkills(
        nodeId,
        skills,
        skillRecords,
      );
      return formatNodeCount(installed, total);
    },
    [skills, skillRecords],
  );
  const installedSkills = useMemo(
    () => skills.map((item) => item.skill),
    [skills],
  );

  if (!target) {
    return (
      <div className="target-detail empty">
        <h2>未选择目标</h2>
        <p>从侧栏选择一个目标目录，以查看和管理 Skill。</p>
      </div>
    );
  }

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
          iflytekEndpoints={iflytekEndpoints}
          repos={repos}
          discoverSkills={[]}
          installedSkills={installedSkills}
          skillRecords={skillRecords}
          selectedNodeId={selectedNodeId}
          onSelectNode={setSelectedNodeId}
          nodeCountLabel={nodeCountLabel}
        />

        <div className="skill-list-panel">
          <div className="skill-list-toolbar">
            <div className="skill-list-toolbar-head">
              <div className="skill-list-head-text">
                <h2 className="skill-list-title">{listHeader.title}</h2>
                <p className="skill-list-sub">{listHeader.sub}</p>
              </div>
              {showSyncButton && onOpenSync ? (
                <div className="skill-list-actions">
                  <button type="button" className="secondary-button" onClick={onOpenSync}>
                    从其他目录同步…
                  </button>
                </div>
              ) : null}
            </div>
          </div>

          <div className="target-body target-body-in-panel">
            {sortedValid.length === 0 && sortedInvalid.length === 0 ? (
              <SkillListEmptyState
                title={skills.length === 0 ? '主库中暂无有效 Skill' : '当前来源下暂无 Skill'}
              />
            ) : (
              <div className="target-list-cards">
                {sortedValid.map((item) => (
                  <SkillRow
                    key={item.skill.storageKey}
                    item={item}
                    skillRecords={skillRecords}
                    pending={pendingSkillKey === item.skill.storageKey}
                    onToggle={onToggleSkill}
                    onPreview={onPreviewSkill}
                  />
                ))}
              </div>
            )}

            {sortedInvalid.length > 0 && (
              <section className="target-invalid-section">
                <h3 className="target-section-label">无效 Skill（{sortedInvalid.length}）</h3>
                <div className="target-list-cards invalid-section">
                  {sortedInvalid.map((item) => (
                    <SkillRow
                      key={item.skill.storageKey || item.skill.dirName}
                      item={item}
                      skillRecords={skillRecords}
                      pending={
                        pendingSkillKey ===
                        (item.skill.storageKey || item.skill.dirName)
                      }
                      onToggle={onToggleSkill}
                      onPreview={onPreviewSkill}
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
