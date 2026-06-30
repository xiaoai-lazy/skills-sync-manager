import React from 'react';
import type { Target } from '../model/types';

export type MainView = 'skill-hub' | 'target';

export interface SidebarProps {
  targets: Target[];
  selectedTargetId: string | null;
  mainView: MainView;
  onOpenSkillHub: () => void;
  onSelectTarget: (targetId: string) => void;
  onAddTarget: () => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
}

function Sidebar(props: SidebarProps) {
  const isSkillHubActive = props.mainView === 'skill-hub';

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">Skills Sync</div>

      <nav className="sidebar-nav">
        <button
          type="button"
          className={`nav-item ${isSkillHubActive ? 'active' : ''}`}
          onClick={props.onOpenSkillHub}
        >
          <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z" />
            <path d="M5 19h14" />
          </svg>
          Skill 中心
        </button>
      </nav>

      <div className="sidebar-section">
        <div className="section-label">目标目录</div>
        {props.targets.length === 0 ? (
          <div className="sidebar-empty">暂无目标目录</div>
        ) : (
          <ul className="target-list">
            {props.targets.map((target) => {
              const isSelected =
                props.mainView === 'target' && target.id === props.selectedTargetId;
              return (
                <li
                  key={target.id}
                  className={`target-item ${isSelected ? 'selected' : ''}`}
                  onClick={() => props.onSelectTarget(target.id)}
                >
                  <span className="target-dot" />
                  <span className="target-name" title={target.skillsDir}>
                    {target.name}
                  </span>
                  <div className="target-actions">
                    <button
                      type="button"
                      className="icon-button"
                      onClick={(e) => {
                        e.stopPropagation();
                        props.onEditTarget(target);
                      }}
                      aria-label={`Edit target ${target.name}`}
                    >
                      ✎
                    </button>
                    <button
                      type="button"
                      className="icon-button danger-button"
                      onClick={(e) => {
                        e.stopPropagation();
                        props.onDeleteTarget(target);
                      }}
                      aria-label={`Delete target ${target.name}`}
                    >
                      🗑
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
        <div className="sidebar-footer">
          <button
            type="button"
            className="btn-add-target"
            onClick={props.onAddTarget}
            aria-label="Add target"
          >
            + 添加目标
          </button>
        </div>
      </div>
    </aside>
  );
}

export default Sidebar;
