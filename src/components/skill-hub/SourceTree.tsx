import { useMemo, type ReactNode } from 'react';
import type {
  DiscoverableSkill,
  IflytekSkillHubEndpoint,
  SkillHubEndpoint,
  SkillRecord,
  SkillRepo,
  SkillView,
} from '../../model/types';
import {
  ALL_NODE_ID,
  LOCAL_NODE_ID,
  hasLocalInstalledSkills,
  hubEndpointVisible,
  hubRootNodeId,
  iflytekRootNodeId,
  repoNodeId,
  repoVisible,
} from './sourceTreeUtils';

export type HubTab = 'installed' | 'discover';

export interface SourceTreeProps {
  tab: HubTab;
  endpoints: SkillHubEndpoint[];
  /** Optional until SkillHubPage wires iFlytek endpoints (Task 9). */
  iflytekEndpoints?: IflytekSkillHubEndpoint[];
  repos: SkillRepo[];
  discoverSkills: DiscoverableSkill[];
  installedSkills: SkillView[];
  skillRecords: Record<string, SkillRecord>;
  selectedNodeId: string;
  onSelectNode: (nodeId: string) => void;
  nodeCountLabel?: (nodeId: string) => string | null | undefined;
}

function SourceTree(props: SourceTreeProps) {
  const {
    tab,
    endpoints,
    iflytekEndpoints = [],
    repos,
    installedSkills,
    skillRecords,
    selectedNodeId,
    onSelectNode,
    nodeCountLabel,
  } = props;

  const visibleEndpoints = useMemo(
    () => endpoints.filter((endpoint) => hubEndpointVisible(endpoint, skillRecords)),
    [endpoints, skillRecords],
  );

  const visibleIflytekEndpoints = useMemo(
    () => iflytekEndpoints.filter((endpoint) => hubEndpointVisible(endpoint, skillRecords)),
    [iflytekEndpoints, skillRecords],
  );

  const visibleRepos = useMemo(
    () => repos.filter((repo) => repoVisible(repo, skillRecords)),
    [repos, skillRecords],
  );

  const showLocal =
    tab === 'installed' && hasLocalInstalledSkills(installedSkills, skillRecords);

  const renderRow = (
    nodeId: string,
    label: string,
    options: {
      selectedClass?: 'selected' | 'hub-selected';
      muted?: boolean;
      title?: string;
      icon: ReactNode;
      onClick?: () => void;
    },
  ) => {
    const selected = selectedNodeId === nodeId;
    const rowClass = [
      'tree-row',
      selected ? (options.selectedClass ?? 'selected') : '',
      options.muted ? 'muted' : '',
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div
        className={rowClass}
        role="treeitem"
        aria-selected={selected}
        title={options.title ?? label}
        onClick={options.onClick}
      >
        <span className="tree-icon" aria-hidden>
          {options.icon}
        </span>
        <span className="tree-label">{label}</span>
        {nodeCountLabel ? (
          (() => {
            const countLabel = nodeCountLabel(nodeId);
            return countLabel ? (
              <span className="tree-count">{countLabel}</span>
            ) : null;
          })()
        ) : null}
      </div>
    );
  };

  const allIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <rect x="3" y="3" width="7" height="7" />
      <rect x="14" y="3" width="7" height="7" />
      <rect x="3" y="14" width="7" height="7" />
      <rect x="14" y="14" width="7" height="7" />
    </svg>
  );

  const hubIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
    </svg>
  );

  const gitHubIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
    </svg>
  );

  const gitLabIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M12 2L2 7l10 15L22 7z" />
    </svg>
  );

  const folderIcon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
    </svg>
  );

  return (
    <div className="source-tree-panel">
      <div className="source-tree-header">来源</div>
      <ul className="source-tree" role="tree">
        <li className="tree-node">
          {renderRow(ALL_NODE_ID, '全部', {
            icon: allIcon,
            onClick: () => onSelectNode(ALL_NODE_ID),
          })}
        </li>

        {visibleEndpoints.map((endpoint) => {
          const rootId = hubRootNodeId(endpoint.id);
          const muted = !endpoint.enabled;

          return (
            <li key={endpoint.id} className="tree-node">
              {renderRow(rootId, endpoint.name, {
                muted,
                title: muted ? `${endpoint.name}（发现已关闭）` : endpoint.name,
                icon: hubIcon,
                onClick: () => onSelectNode(rootId),
              })}
            </li>
          );
        })}

        {visibleIflytekEndpoints.map((endpoint) => {
          const rootId = iflytekRootNodeId(endpoint.id);
          const muted = !endpoint.enabled;

          return (
            <li key={`iflytek-${endpoint.id}`} className="tree-node">
              {renderRow(rootId, endpoint.name, {
                muted,
                title: muted ? `${endpoint.name}（发现已关闭）` : endpoint.name,
                icon: hubIcon,
                onClick: () => onSelectNode(rootId),
              })}
            </li>
          );
        })}

        {visibleRepos.map((repo) => {
          const nodeId = repoNodeId(repo.host, repo.projectPath);
          const label =
            repo.provider === 'gitlab'
              ? `${repo.host}/${repo.projectPath}`
              : repo.projectPath || `${repo.owner}/${repo.name}`;
          const muted = !repo.enabled;

          return (
            <li key={nodeId} className="tree-node">
              {renderRow(nodeId, label, {
                muted,
                title: muted ? `${label}（发现已关闭）` : label,
                icon: repo.provider === 'gitlab' ? gitLabIcon : gitHubIcon,
                onClick: () => onSelectNode(nodeId),
              })}
            </li>
          );
        })}

        {showLocal && (
          <li className="tree-node">
            {renderRow(LOCAL_NODE_ID, '本地导入', {
              icon: folderIcon,
              onClick: () => onSelectNode(LOCAL_NODE_ID),
            })}
          </li>
        )}
      </ul>
    </div>
  );
}

export default SourceTree;
