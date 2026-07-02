import type { Project, Target } from '../model/types';
import type { MainView } from './Sidebar';
import TargetRow from './TargetRow';

export interface ProjectTreeProps {
  projects: Project[];
  targets: Target[];
  expandedProjectIds: ReadonlySet<string>;
  selectedProjectId: string | null;
  selectedTargetId: string | null;
  mainView: MainView;
  onToggleProject: (projectId: string) => void;
  onAddProjectTarget: (projectId: string) => void;
  onEditProject: (project: Project) => void;
  onDeleteProject: (project: Project) => void;
  onSelectTarget: (targetId: string) => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
}

function ProjectTree(props: ProjectTreeProps) {
  if (props.projects.length === 0) {
    return <div className="sidebar-empty">暂无项目</div>;
  }

  return (
    <ul className="project-tree">
      {props.projects.map((project) => {
        const expanded = props.expandedProjectIds.has(project.id);
        const isActive = props.selectedProjectId === project.id;
        const projectTargets = props.targets.filter(
          (target) => target.projectId === project.id
        );

        return (
          <li key={project.id} className="project-node project-row">
            <div
              className={`project-header ${expanded ? 'expanded' : ''} ${isActive ? 'active' : ''}`}
              onClick={() => props.onToggleProject(project.id)}
            >
              <span className="project-chevron" aria-hidden="true">
                ▶
              </span>
              <span className="project-folder" aria-hidden="true">
                📁
              </span>
              <span className="target-name" title={project.rootPath}>
                {project.name}
              </span>
              <div className="target-actions project-header-actions">
                <button
                  type="button"
                  className="icon-button section-add-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onAddProjectTarget(project.id);
                  }}
                  aria-label={`Add target to ${project.name}`}
                  title="添加目标"
                >
                  +
                </button>
                <button
                  type="button"
                  className="icon-button"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onEditProject(project);
                  }}
                  aria-label={`Edit project ${project.name}`}
                >
                  ✎
                </button>
                <button
                  type="button"
                  className="icon-button danger-button"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onDeleteProject(project);
                  }}
                  aria-label={`Delete project ${project.name}`}
                >
                  🗑
                </button>
              </div>
            </div>
            <ul className={`project-children ${expanded ? 'visible' : ''}`}>
              {projectTargets.map((target) => {
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
          </li>
        );
      })}
    </ul>
  );
}

export default ProjectTree;
