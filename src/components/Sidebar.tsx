import type { Project, Target } from '../model/types';
import ProjectTree from './ProjectTree';
import TargetRow from './TargetRow';

export type MainView = 'skill-hub' | 'target';

export interface SidebarProps {
  targets: Target[];
  projects: Project[];
  expandedProjectIds: ReadonlySet<string>;
  selectedTargetId: string | null;
  selectedProjectId: string | null;
  mainView: MainView;
  onOpenSkillHub: () => void;
  onSelectTarget: (targetId: string) => void;
  onToggleProject: (projectId: string) => void;
  onAddGlobalTarget: () => void;
  onAddProject: () => void;
  onAddProjectTarget: (projectId: string) => void;
  onEditTarget: (target: Target) => void;
  onEditProject: (project: Project) => void;
  onDeleteTarget: (target: Target) => void;
  onDeleteProject: (project: Project) => void;
}

function Sidebar(props: SidebarProps) {
  const isSkillHubActive = props.mainView === 'skill-hub';
  const globalTargets = props.targets.filter((target) => target.scope === 'global');

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

      <div className="sidebar-block">
        <div className="section-label">
          <span>Agent</span>
          <div className="section-label-actions">
            <span className="section-badge">{globalTargets.length}</span>
            <button
              type="button"
              className="section-add-btn"
              onClick={props.onAddGlobalTarget}
              aria-label="Add global target"
              title="添加目标"
            >
              +
            </button>
          </div>
        </div>
        {globalTargets.length === 0 ? (
          <div className="sidebar-empty">暂无用户级目标</div>
        ) : (
          <ul className="target-list">
            {globalTargets.map((target) => {
              const isSelected =
                props.mainView === 'target' && target.id === props.selectedTargetId;
              return (
                <TargetRow
                  key={target.id}
                  target={target}
                  isSelected={isSelected}
                  onSelect={() => props.onSelectTarget(target.id)}
                  onEdit={props.onEditTarget}
                  onDelete={props.onDeleteTarget}
                />
              );
            })}
          </ul>
        )}
      </div>

      <div className="sidebar-block sidebar-block-projects">
        <div className="section-label">
          <span>项目</span>
          <div className="section-label-actions">
            <span className="section-badge">{props.projects.length}</span>
            <button
              type="button"
              className="section-add-btn"
              onClick={props.onAddProject}
              aria-label="Add project"
              title="添加项目"
            >
              +
            </button>
          </div>
        </div>
        <ProjectTree
          projects={props.projects}
          targets={props.targets}
          expandedProjectIds={props.expandedProjectIds}
          selectedProjectId={props.selectedProjectId}
          selectedTargetId={props.selectedTargetId}
          mainView={props.mainView}
          onToggleProject={props.onToggleProject}
          onAddProjectTarget={props.onAddProjectTarget}
          onEditProject={props.onEditProject}
          onDeleteProject={props.onDeleteProject}
          onSelectTarget={props.onSelectTarget}
          onEditTarget={props.onEditTarget}
          onDeleteTarget={props.onDeleteTarget}
        />
      </div>
    </aside>
  );
}

export default Sidebar;
