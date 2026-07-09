import React from 'react';
import type { DiscoverableSkill, SkillView } from '../../model/types';

export type SkillCardSkill = SkillView | DiscoverableSkill;

export function isDiscoverableSkill(skill: SkillCardSkill): skill is DiscoverableSkill {
  return 'key' in skill;
}

export interface SkillCardProps {
  skill: SkillCardSkill;
  mode: 'installed' | 'discover';
  hasUpdate?: boolean;
  sourceMissing?: boolean;
  selected?: boolean;
  sourceLabel: string;
  onSelect?: (selected: boolean) => void;
  onInstall?: () => void;
  onUpdate?: () => void;
  onDelete?: () => void;
}

function getCardId(skill: SkillCardSkill): string {
  return isDiscoverableSkill(skill) ? skill.key : skill.dirName;
}

function getTitle(skill: SkillCardSkill): string {
  if (isDiscoverableSkill(skill)) return skill.name;
  if (!skill.valid) return `(无效) ${skill.name ?? skill.dirName}`;
  return skill.name ?? skill.dirName;
}

function getDescription(skill: SkillCardSkill): string {
  if (isDiscoverableSkill(skill)) return skill.description;
  if (!skill.valid && skill.validationErrors.length > 0) {
    return skill.validationErrors.join('；');
  }
  return skill.description ?? '';
}

function getSourceMeta(sourceLabel: string, skill: SkillCardSkill): string {
  if (isDiscoverableSkill(skill) && skill.source === 'gitlab' && skill.repoHost) {
    return `GitLab · ${skill.repoHost}`;
  }
  if (isDiscoverableSkill(skill) && skill.source === 'skillhub' && skill.hubSkillGroup) {
    return `Skill Hub · ${skill.hubSkillGroup}`;
  }
  return sourceLabel;
}

function SkillCard(props: SkillCardProps) {
  const {
    skill,
    mode,
    hasUpdate = false,
    sourceMissing = false,
    selected = false,
    sourceLabel,
    onSelect,
    onInstall,
    onUpdate,
    onDelete,
  } = props;

  const id = getCardId(skill);
  const title = getTitle(skill);
  const desc = getDescription(skill);
  const invalid = !isDiscoverableSkill(skill) && !skill.valid;
  const isDiscover = mode === 'discover';
  const sourceMeta = getSourceMeta(sourceLabel, skill);
  const showUpdate = hasUpdate && !invalid && !sourceMissing;

  let actions: React.ReactNode = null;
  if (isDiscover) {
    actions = (
      <button type="button" className="btn-sm btn-primary" onClick={onInstall}>
        安装
      </button>
    );
  } else if (invalid) {
    actions = (
      <button type="button" className="btn-sm btn-danger" onClick={onDelete}>
        删除
      </button>
    );
  } else if (showUpdate) {
    actions = (
      <>
        <button type="button" className="btn-sm btn-primary" onClick={onUpdate}>
          更新
        </button>
        <button type="button" className="btn-sm btn-danger" onClick={onDelete}>
          删除
        </button>
      </>
    );
  } else {
    actions = (
      <button type="button" className="btn-sm btn-danger" onClick={onDelete}>
        删除
      </button>
    );
  }

  const handleCardClick = () => {
    if (!isDiscover || !onSelect) return;
    onSelect(!selected);
  };

  const showBadges = showUpdate || invalid || sourceMissing;

  return (
    <article
      className={`skill-card${selected ? ' selected' : ''}${invalid ? ' invalid' : ''}${sourceMissing ? ' source-missing' : ''}`}
      data-id={id}
      onClick={handleCardClick}
    >
      {isDiscover && onSelect && (
        <input
          type="checkbox"
          className="card-check"
          checked={selected}
          onChange={(e) => {
            e.stopPropagation();
            onSelect(e.target.checked);
          }}
          onClick={(e) => e.stopPropagation()}
          aria-label={`选择 ${title}`}
        />
      )}
      <div className="skill-card-header">
        <h3 className="skill-card-title">{title}</h3>
        {showBadges ? (
          <div className="skill-card-badges">
            {sourceMissing && (
              <span className="badge badge-source-missing" title="远程源中已不存在该 Skill，本地副本仍保留">
                源缺失
              </span>
            )}
            {showUpdate && <span className="badge badge-update">有更新</span>}
            {invalid && <span className="badge badge-invalid">无效</span>}
          </div>
        ) : null}
      </div>
      <p className="skill-card-desc">{desc || '—'}</p>
      <div className="skill-card-footer">
        <div className="skill-card-meta">{sourceMeta}</div>
        <div
          className="skill-card-actions"
          onClick={(e) => e.stopPropagation()}
        >
          {actions}
        </div>
      </div>
    </article>
  );
}

export default SkillCard;
