import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  checkSkillUpdates,
  discoverSkills,
  getSkillRepos,
  installHubSkill,
  listHubGroups,
  listSkillHubEndpoints,
  scanMainLibrary,
  updateAllSkills,
  updateSkill,
} from '../../api/skillHub';
import type {
  DiscoverableSkill,
  SkillHubEndpoint,
  SkillHubLocalState,
  SkillRecord,
  SkillUpdateInfo,
  SkillView,
  StartupRefreshSettings,
} from '../../model/types';
import { errorMessage } from '../../utils/errorMessage';
import { isInProgressError, isHubSkillGoneError } from '../../utils/ipcError';
import InstallConfirmDialog from './InstallConfirmDialog';
import SkillListEmptyState from './SkillListEmptyState';
import SkillCard from './SkillCard';
import SmartPasteBar from './SmartPasteBar';
import SourceManageDrawer from './SourceManageDrawer';
import SourceTree from './SourceTree';
import UploadToHubDialog from './UploadToHubDialog';
import {
  skillSourceLabelForDiscoverable,
  skillSourceLabelForView,
} from '../../utils/skillSourceLabel';
import {
  ALL_HUB_GROUP,
  ALL_NODE_ID,
  hubGroupsForEndpoint,
  hubRootNodeId,
  isEnabledHubRootNode,
  isHubRootNode,
  matchesDiscoverNode,
  matchesInstalledNode,
  countDiscoverForNode,
  countInstalledForNode,
  dedupeInstalledSkills,
  nodeTitle,
  parseHubNodeId,
  resolveEffectiveFilterNodeId,
  resolveSkillRecord,
  findPendingUpdate,
  skillHasPendingUpdate,
  pendingUpdateIdentifier,
} from './sourceTreeUtils';

type HubTab = 'installed' | 'discover';
type InstalledChip = 'all' | 'updates';

export interface SkillHubPageProps {
  mainSkillsDir: string | null;
  hubState: SkillHubLocalState;
  discoverSkills: DiscoverableSkill[];
  pendingUpdates: SkillUpdateInfo[];
  skillRecords?: Record<string, SkillRecord>;
  skillHubEndpoints?: SkillHubEndpoint[];
  startupRefreshSettings: StartupRefreshSettings;
  onStartupRefreshSettingsChange?: (settings: StartupRefreshSettings) => void;
  /** Optional fallback when onRefreshHub is absent; receives skills only (no skillRecords write-back). */
  onHubSkillsRefresh?: (skills: SkillView[]) => void;
  onDiscoverSkillsChange: (skills: DiscoverableSkill[]) => void;
  onPendingUpdatesChange: (updates: SkillUpdateInfo[]) => void;
  onDeleteMainSkill: (storageKey: string, displayName: string) => void;
  onSetMainSkillsDir: () => void;
  onRefreshHub?: () => Promise<void>;
  onToast?: (message: string) => void;
  onError?: (error: unknown) => void;
}

function matchesSearch(
  query: string,
  parts: Array<string | null | undefined>,
): boolean {
  if (!query) return true;
  const hay = parts.filter(Boolean).join(' ').toLowerCase();
  return hay.includes(query.toLowerCase());
}

function SkillHubPage(props: SkillHubPageProps) {
  const {
    mainSkillsDir,
    hubState,
    discoverSkills: discoverList,
    pendingUpdates,
    skillRecords,
    skillHubEndpoints: initialEndpoints,
    startupRefreshSettings,
    onStartupRefreshSettingsChange,
    onHubSkillsRefresh,
    onDiscoverSkillsChange,
    onPendingUpdatesChange,
    onDeleteMainSkill,
    onSetMainSkillsDir,
    onRefreshHub,
    onToast,
    onError,
  } = props;

  const [tab, setTab] = useState<HubTab>('installed');
  const [installedChip, setInstalledChip] = useState<InstalledChip>('all');
  const [search, setSearch] = useState('');
  const [selectedNodeId, setSelectedNodeId] = useState(ALL_NODE_ID);
  const [selectedHubGroup, setSelectedHubGroup] = useState(ALL_HUB_GROUP);
  const [hubGroupsFromServer, setHubGroupsFromServer] = useState<string[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [sourceDrawerOpen, setSourceDrawerOpen] = useState(false);
  const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
  const [installDialogOpen, setInstallDialogOpen] = useState(false);
  const [pendingInstall, setPendingInstall] = useState<DiscoverableSkill[]>([]);
  const [installing, setInstalling] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updatingAll, setUpdatingAll] = useState(false);
  const [refreshingDiscover, setRefreshingDiscover] = useState(false);
  const [hubInstalling, setHubInstalling] = useState(false);
  const [endpoints, setEndpoints] = useState<SkillHubEndpoint[]>(initialEndpoints ?? []);
  const [repos, setRepos] = useState<import('../../model/types').SkillRepo[]>([]);

  const discoverInFlight = useRef(false);
  const checkInFlight = useRef(false);

  const refreshHubState = useCallback(async () => {
    if (onRefreshHub) {
      await onRefreshHub();
      return;
    }
    const next = await scanMainLibrary();
    onHubSkillsRefresh?.(next.skills);
  }, [onRefreshHub, onHubSkillsRefresh]);

  useEffect(() => {
    void refreshHubState().catch((err) => {
      onError?.(errorMessage(err));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (initialEndpoints && initialEndpoints.length > 0) {
      setEndpoints(initialEndpoints);
      return;
    }
    // `[]` / undefined from appState must not skip the IPC load — SourceManageDrawer
    // was the only place that refreshed endpoints, which hid configured hubs until opened.
    void listSkillHubEndpoints()
      .then(setEndpoints)
      .catch((err) => onError?.(errorMessage(err)));
  }, [initialEndpoints, onError]);

  useEffect(() => {
    void getSkillRepos()
      .then(setRepos)
      .catch((err) => onError?.(errorMessage(err)));
  }, [onError]);

  const handleCheckUpdates = async () => {
    if (checkInFlight.current) {
      onToast?.('正在刷新，请稍候');
      return;
    }
    checkInFlight.current = true;
    setCheckingUpdates(true);
    try {
      const updates = await checkSkillUpdates();
      onPendingUpdatesChange(updates);
      await refreshHubState();
      onToast?.(updates.length > 0 ? `发现 ${updates.length} 个更新` : '暂无更新');
    } catch (err) {
      if (isInProgressError(err)) {
        onToast?.('正在刷新，请稍候');
      } else {
        onError?.(errorMessage(err));
      }
    } finally {
      checkInFlight.current = false;
      setCheckingUpdates(false);
    }
  };

  const handleUpdateAll = async () => {
    if (pendingUpdates.length === 0 || updatingAll) return;
    setUpdatingAll(true);
    try {
      const result = await updateAllSkills();
      const nextUpdates = pendingUpdates.filter(
        (u) => !result.updated.includes(u.dirName),
      );
      onPendingUpdatesChange(nextUpdates);
      await refreshHubState();
      if (result.failed.length > 0) {
        onError?.(`${result.failed.length} 个 Skill 更新失败`);
      } else {
        onToast?.('已全部更新');
      }
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setUpdatingAll(false);
    }
  };

  const handleRefreshDiscover = async () => {
    if (discoverInFlight.current) {
      onToast?.('正在刷新，请稍候');
      return;
    }
    discoverInFlight.current = true;
    setRefreshingDiscover(true);
    try {
      const result = await discoverSkills(true);
      onDiscoverSkillsChange(result.skills);
      if (result.warnings.length > 0) {
        onToast?.(
          result.warnings.length === 1
            ? result.warnings[0]
            : `部分来源不可用（${result.warnings.length} 个），其余已刷新`,
        );
      } else {
        onToast?.('列表已刷新');
      }
    } catch (err) {
      if (isInProgressError(err)) {
        onToast?.('正在刷新，请稍候');
      } else {
        onError?.(errorMessage(err));
      }
    } finally {
      discoverInFlight.current = false;
      setRefreshingDiscover(false);
    }
  };

  const handleUpdateSkill = async (skill: SkillView) => {
    const identifier = pendingUpdateIdentifier(skill, pendingUpdates);
    try {
      await updateSkill(identifier);
      onPendingUpdatesChange(
        pendingUpdates.filter((update) => findPendingUpdate(skill, [update]) === undefined),
      );
      await refreshHubState();
      onToast?.('已更新');
    } catch (err) {
      if (isHubSkillGoneError(err)) {
        onPendingUpdatesChange(
          pendingUpdates.filter((update) => findPendingUpdate(skill, [update]) === undefined),
        );
        await refreshHubState().catch(() => {});
        return;
      }
      onError?.(errorMessage(err));
    }
  };

  const handleInstallSkill = async (skill: DiscoverableSkill) => {
    setHubInstalling(true);
    try {
      await installHubSkill(skill);
      onDiscoverSkillsChange(discoverList.filter((s) => s.key !== skill.key));
      setSelectedKeys((prev) => {
        const next = new Set(prev);
        next.delete(skill.key);
        return next;
      });
      await refreshHubState();
      onToast?.('已安装到主库');
    } catch (err) {
      if (isHubSkillGoneError(err)) {
        onDiscoverSkillsChange(discoverList.filter((s) => s.key !== skill.key));
        setSelectedKeys((prev) => {
          const next = new Set(prev);
          next.delete(skill.key);
          return next;
        });
        await refreshHubState().catch(() => {});
        return;
      }
      onError?.(errorMessage(err));
      await refreshHubState().catch(() => {});
    } finally {
      setHubInstalling(false);
    }
  };

  const openInstallDialog = (skills: DiscoverableSkill[]) => {
    setPendingInstall(skills);
    setInstallDialogOpen(true);
  };

  const handleConfirmInstall = async () => {
    setInstalling(true);
    const doneKeys: string[] = [];
    let installedCount = 0;
    try {
      for (const skill of pendingInstall) {
        try {
          await installHubSkill(skill);
          doneKeys.push(skill.key);
          installedCount += 1;
        } catch (err) {
          if (isHubSkillGoneError(err)) {
            doneKeys.push(skill.key);
            continue;
          }
          throw err;
        }
      }
      if (doneKeys.length > 0) {
        onDiscoverSkillsChange(discoverList.filter((s) => !doneKeys.includes(s.key)));
        setSelectedKeys((prev) => {
          const next = new Set(prev);
          doneKeys.forEach((k) => next.delete(k));
          return next;
        });
        await refreshHubState();
      }
      if (installedCount > 0) {
        onToast?.(
          installedCount > 1 ? `已安装 ${installedCount} 个 Skill 到主库` : '已安装到主库',
        );
      }
    } catch (err) {
      if (doneKeys.length > 0) {
        onDiscoverSkillsChange(discoverList.filter((s) => !doneKeys.includes(s.key)));
        setSelectedKeys((prev) => {
          const next = new Set(prev);
          doneKeys.forEach((k) => next.delete(k));
          return next;
        });
      }
      onError?.(errorMessage(err));
      await refreshHubState().catch(() => {});
    } finally {
      setInstallDialogOpen(false);
      setPendingInstall([]);
      setInstalling(false);
    }
  };

  const installedRecords = skillRecords ?? hubState.skillRecords;

  const dedupedInstalled = useMemo(
    () => dedupeInstalledSkills(hubState.skills),
    [hubState.skills],
  );

  const effectiveFilterNodeId = useMemo(
    () => resolveEffectiveFilterNodeId(selectedNodeId, selectedHubGroup, endpoints),
    [selectedNodeId, selectedHubGroup, endpoints],
  );

  const showHubGroupFilter = isHubRootNode(selectedNodeId, endpoints);
  const selectedHubEndpointId = useMemo(() => {
    const hub = parseHubNodeId(selectedNodeId);
    if (hub && !hub.group) return hub.endpointId;
    return null;
  }, [selectedNodeId]);

  const localHubGroups = useMemo(() => {
    if (!selectedHubEndpointId) return [];
    return hubGroupsForEndpoint(selectedHubEndpointId, discoverList, installedRecords);
  }, [selectedHubEndpointId, discoverList, installedRecords]);

  const availableHubGroups = useMemo(() => {
    const merged = new Set([...hubGroupsFromServer, ...localHubGroups]);
    return [...merged].sort();
  }, [hubGroupsFromServer, localHubGroups]);

  useEffect(() => {
    if (!selectedHubEndpointId) {
      setHubGroupsFromServer([]);
      return;
    }

    let cancelled = false;
    void listHubGroups(selectedHubEndpointId)
      .then((groups) => {
        if (!cancelled) setHubGroupsFromServer(groups);
      })
      .catch((err) => {
        if (!cancelled) {
          setHubGroupsFromServer([]);
          onError?.(errorMessage(err));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedHubEndpointId, discoverList, onError]);

  const handleSelectNode = (nodeId: string) => {
    const hub = parseHubNodeId(nodeId);
    if (hub?.group) {
      setSelectedNodeId(hubRootNodeId(hub.endpointId));
      setSelectedHubGroup(hub.group);
      return;
    }
    setSelectedNodeId(nodeId);
    setSelectedHubGroup(ALL_HUB_GROUP);
  };

  useEffect(() => {
    if (selectedHubGroup === ALL_HUB_GROUP) return;
    if (!availableHubGroups.includes(selectedHubGroup)) {
      setSelectedHubGroup(ALL_HUB_GROUP);
    }
  }, [availableHubGroups, selectedHubGroup]);

  const filteredInstalled = useMemo(() => {
    return dedupedInstalled.filter((skill) => {
      const hasUpdate = skillHasPendingUpdate(skill, pendingUpdates);
      if (installedChip === 'updates' && !hasUpdate) return false;
      if (
        !matchesInstalledNode(
          effectiveFilterNodeId,
          skill.dirName,
          resolveSkillRecord(skill, installedRecords),
          skill,
        )
      ) {
        return false;
      }
      return matchesSearch(search, [
        skill.name,
        skill.description,
        skill.dirName,
        skill.path,
        skill.storageKey,
      ]);
    });
  }, [
    dedupedInstalled,
    installedChip,
    pendingUpdates,
    search,
    effectiveFilterNodeId,
    installedRecords,
  ]);

  const filteredDiscover = useMemo(() => {
    return discoverList.filter((skill) => {
      if (!matchesDiscoverNode(effectiveFilterNodeId, skill)) return false;
      return matchesSearch(search, [
        skill.name,
        skill.description,
        skill.installDirName,
        skill.directory,
        `${skill.repoOwner}/${skill.repoName}`,
      ]);
    });
  }, [discoverList, search, effectiveFilterNodeId]);

  const toggleSelection = (key: string, selected: boolean) => {
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      if (selected) next.add(key);
      else next.delete(key);
      return next;
    });
  };

  const clearSelection = () => setSelectedKeys(new Set());

  const pendingCount = pendingUpdates.length;
  const installedCountForNode = useMemo(
    () => countInstalledForNode(effectiveFilterNodeId, dedupedInstalled, installedRecords),
    [effectiveFilterNodeId, dedupedInstalled, installedRecords],
  );
  const discoverCountForNode = useMemo(
    () => countDiscoverForNode(effectiveFilterNodeId, discoverList),
    [effectiveFilterNodeId, discoverList],
  );
  const showBatchBar = tab === 'discover' && selectedKeys.size > 0;
  const discoverBusy = refreshingDiscover || discoverInFlight.current;
  const checkBusy = checkingUpdates || checkInFlight.current;
  const hubBusy = installing || hubInstalling || checkingUpdates || updatingAll || refreshingDiscover;
  const hubBusyLabel =
    installing || hubInstalling
      ? '正在安装…'
      : checkingUpdates
        ? '正在检查更新…'
        : updatingAll
          ? '正在更新…'
          : '正在刷新列表…';

  const listHeader = nodeTitle(selectedNodeId, endpoints, repos);
  const enabledHubs = endpoints.filter((e) => e.enabled);
  const showUploadButton =
    tab === 'installed' &&
    enabledHubs.length > 0 &&
    isEnabledHubRootNode(selectedNodeId, endpoints);
  const selectedHubEndpoint = endpoints.find(
    (e) => e.id === selectedNodeId.replace(/^hub:/, '').split(':')[0],
  );

  const listEmptyState = useMemo((): {
    title: string;
    description: string;
    actionLabel?: string;
    action?: 'clearSearch' | 'resetGroup' | 'refreshDiscover' | 'showAllInstalled';
  } => {
    const query = search.trim();
    if (tab === 'discover' && discoverList.length === 0) {
      return {
        title: '暂无可安装 Skill',
        description: '点击「刷新列表」从 Hub 与已配置来源拉取最新 Skill。',
        actionLabel: '刷新列表',
        action: 'refreshDiscover',
      };
    }
    if (selectedHubGroup !== ALL_HUB_GROUP) {
      if (tab === 'installed') {
        return {
          title: `「${selectedHubGroup}」暂无已安装 Skill`,
          description: '可切换到「可安装」安装，或查看全部分组。',
          actionLabel: '查看全部分组',
          action: 'resetGroup',
        };
      }
      return {
        title: `「${selectedHubGroup}」暂无可安装 Skill`,
        description: '可上传 Skill 到 Hub，或切换到其他分组。',
        actionLabel: '查看全部分组',
        action: 'resetGroup',
      };
    }
    if (query) {
      return {
        title: '未找到匹配 Skill',
        description: `没有与「${query}」匹配的结果，试试其他关键词。`,
        actionLabel: '清空搜索',
        action: 'clearSearch',
      };
    }
    if (tab === 'installed' && installedChip === 'updates') {
      return {
        title: pendingCount > 0 ? '当前筛选下暂无有更新的 Skill' : '暂无有更新的 Skill',
        description:
          pendingCount > 0
            ? '试试切换分组、来源或清空搜索条件。'
            : '所有 Skill 均已是最新版本。',
        actionLabel: '查看全部',
        action: 'showAllInstalled',
      };
    }
    if (tab === 'installed') {
      return {
        title: '暂无已安装 Skill',
        description: '当前来源下还没有已安装的 Skill，可切换到「可安装」进行安装。',
      };
    }
    return {
      title: '暂无可安装 Skill',
      description: '当前来源下没有可安装的 Skill。',
      actionLabel: '刷新列表',
      action: 'refreshDiscover',
    };
  }, [
    tab,
    discoverList.length,
    selectedHubGroup,
    search,
    installedChip,
    pendingCount,
  ]);

  const handleEmptyAction = () => {
    switch (listEmptyState.action) {
      case 'clearSearch':
        setSearch('');
        break;
      case 'resetGroup':
        setSelectedHubGroup(ALL_HUB_GROUP);
        break;
      case 'refreshDiscover':
        void handleRefreshDiscover();
        break;
      case 'showAllInstalled':
        setInstalledChip('all');
        break;
      default:
        break;
    }
  };

  return (
    <section className="skill-hub-page">
      {hubBusy && (
        <div className="hub-busy-overlay" aria-live="polite">
          {hubBusyLabel}
        </div>
      )}
      <div className="hub-body">
        <div className="hub-hero hub-hero-compact">
          <div className="hub-hero-row">
            <h1>Skill 中心</h1>
            <div className="hub-stat-row">
              <span className="pill hub-pill">{hubState.validCount} 有效</span>
              {hubState.invalidCount > 0 && (
                <span className="pill hub-pill warn">{hubState.invalidCount} 无效</span>
              )}
              {pendingCount > 0 && (
                <span className="pill hub-pill update">{pendingCount} 待更新</span>
              )}
            </div>
            <div className="hub-path-inline" title={mainSkillsDir ?? undefined}>
              <span className="hub-path-text">{mainSkillsDir ?? '未设置主库目录'}</span>
              <button
                type="button"
                className="dir-path-edit"
                aria-label="更改主库目录"
                onClick={onSetMainSkillsDir}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  aria-hidden="true"
                >
                  <path d="M12 20h9" />
                  <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" />
                </svg>
              </button>
            </div>
            <div className="hub-actions">
              <button
                type="button"
                className="btn-sm"
                onClick={() => void handleCheckUpdates()}
                disabled={checkBusy || updatingAll}
              >
                {checkBusy ? '检查更新…' : '检查更新'}
              </button>
              <button
                type="button"
                className="btn-sm btn-primary"
                onClick={() => void handleUpdateAll()}
                disabled={pendingCount === 0 || updatingAll || checkBusy}
              >
                {updatingAll ? '更新中…' : `全部更新 (${pendingCount})`}
              </button>
              <button type="button" className="btn-sm" onClick={() => setSourceDrawerOpen(true)}>
                来源管理
              </button>
            </div>
          </div>
        </div>

        <SmartPasteBar onInstall={handleInstallSkill} onError={onError} />

        <div className="hub-split">
          <SourceTree
            tab={tab}
            endpoints={endpoints}
            repos={repos}
            discoverSkills={discoverList}
            installedSkills={hubState.skills}
            skillRecords={installedRecords}
            selectedNodeId={selectedNodeId}
            onSelectNode={handleSelectNode}
          />

          <div className="skill-list-panel">
            <div className="skill-list-toolbar">
              <div className="skill-list-toolbar-head">
                <div className="skill-list-head-text">
                  <h2 className="skill-list-title">{listHeader.title}</h2>
                  <p className="skill-list-sub">{listHeader.sub}</p>
                </div>
                <div className="skill-list-actions">
                  {tab === 'discover' && (
                    <button
                      type="button"
                      className="btn-sm"
                      onClick={() => void handleRefreshDiscover()}
                      disabled={discoverBusy}
                    >
                      {refreshingDiscover ? '拉取中…' : '刷新列表'}
                    </button>
                  )}
                  {showUploadButton && selectedHubEndpoint && (
                    <button
                      type="button"
                      className="btn-sm btn-hub"
                      onClick={() => setUploadDialogOpen(true)}
                    >
                      上传到 Hub
                    </button>
                  )}
                </div>
              </div>
              <div className="hub-toolbar hub-toolbar-in-panel hub-toolbar-compact">
                <div className="hub-toolbar-search">
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    aria-hidden="true"
                  >
                    <circle cx="11" cy="11" r="8" />
                    <path d="m21 21-4.3-4.3" />
                  </svg>
                  <input
                    type="search"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    placeholder="搜索 Skill…"
                    aria-label="搜索 Skill"
                  />
                </div>
                {showHubGroupFilter && (
                  <div className="hub-filter-control">
                    <span className="hub-filter-control-label" id="hub-group-label">
                      分组
                    </span>
                    <div className="hub-filter-select">
                      <select
                        id="hub-group-select"
                        value={selectedHubGroup}
                        onChange={(e) => setSelectedHubGroup(e.target.value)}
                        aria-labelledby="hub-group-label"
                      >
                        <option value={ALL_HUB_GROUP}>全部</option>
                        {availableHubGroups.map((group) => (
                          <option key={group} value={group}>
                            {group}
                          </option>
                        ))}
                      </select>
                      <svg
                        className="hub-filter-select-chevron"
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        aria-hidden="true"
                      >
                        <path d="m6 9 6 6 6-6" />
                      </svg>
                    </div>
                  </div>
                )}
                <div className="text-tabs" role="tablist">
                  <button
                    type="button"
                    role="tab"
                    className={`text-tab hub-tab${tab === 'installed' ? ' active' : ''}`}
                    aria-selected={tab === 'installed'}
                    onClick={() => {
                      setTab('installed');
                      clearSelection();
                    }}
                  >
                    已安装 ({installedCountForNode})
                  </button>
                  <button
                    type="button"
                    role="tab"
                    className={`text-tab hub-tab${tab === 'discover' ? ' active' : ''}`}
                    aria-selected={tab === 'discover'}
                    onClick={() => {
                      setTab('discover');
                      setInstalledChip('all');
                    }}
                  >
                    可安装 ({discoverCountForNode})
                  </button>
                </div>
                {tab === 'installed' && (
                  <div className="chip-group" id="installedChips">
                    <button
                      type="button"
                      className={`chip filter-chip${installedChip === 'all' ? ' active' : ''}`}
                      onClick={() => setInstalledChip('all')}
                    >
                      全部
                    </button>
                    <button
                      type="button"
                      className={`chip filter-chip${installedChip === 'updates' ? ' active' : ''}`}
                      onClick={() => setInstalledChip('updates')}
                    >
                      有更新 ({pendingCount})
                    </button>
                  </div>
                )}
              </div>
            </div>

            {showBatchBar && (
              <div className="batch-bar">
                <span>已选 {selectedKeys.size} 项</span>
                <div>
                  <button
                    type="button"
                    className="btn-sm btn-primary"
                    onClick={() => {
                      const skills = discoverList.filter((s) => selectedKeys.has(s.key));
                      openInstallDialog(skills);
                    }}
                  >
                    批量安装
                  </button>
                  <button type="button" className="btn-sm btn-ghost" onClick={clearSelection}>
                    取消选择
                  </button>
                </div>
              </div>
            )}

            <div className="skill-grid">
              {tab === 'installed' ? (
                filteredInstalled.length > 0 ? (
                  filteredInstalled.map((skill) => (
                    <SkillCard
                      key={skill.storageKey}
                      skill={skill}
                      mode="installed"
                      hasUpdate={skillHasPendingUpdate(skill, pendingUpdates)}
                      sourceMissing={
                        resolveSkillRecord(skill, installedRecords)?.sourceMissing === true
                      }
                      sourceLabel={skillSourceLabelForView(skill, installedRecords)}
                      onUpdate={() => void handleUpdateSkill(skill)}
                      onDelete={() => {
                        onDeleteMainSkill(skill.storageKey, skill.name ?? skill.dirName);
                      }}
                    />
                  ))
                ) : (
                  <SkillListEmptyState
                    title={listEmptyState.title}
                    description={listEmptyState.description}
                    actionLabel={listEmptyState.actionLabel}
                    onAction={listEmptyState.action ? handleEmptyAction : undefined}
                  />
                )
              ) : filteredDiscover.length > 0 ? (
                filteredDiscover.map((skill) => (
                  <SkillCard
                    key={skill.key}
                    skill={skill}
                    mode="discover"
                    selected={selectedKeys.has(skill.key)}
                    sourceLabel={skillSourceLabelForDiscoverable(skill)}
                    onSelect={(selected) => toggleSelection(skill.key, selected)}
                    onInstall={() => openInstallDialog([skill])}
                  />
                ))
              ) : (
                <SkillListEmptyState
                  title={listEmptyState.title}
                  description={listEmptyState.description}
                  actionLabel={listEmptyState.actionLabel}
                  onAction={listEmptyState.action ? handleEmptyAction : undefined}
                />
              )}
            </div>
          </div>
        </div>
      </div>

      <InstallConfirmDialog
        open={installDialogOpen}
        preview={pendingInstall[0] ?? null}
        batchCount={pendingInstall.length}
        installing={installing}
        onConfirm={() => void handleConfirmInstall()}
        onCancel={() => {
          setInstallDialogOpen(false);
          setPendingInstall([]);
        }}
      />

      <SourceManageDrawer
        open={sourceDrawerOpen}
        onClose={() => setSourceDrawerOpen(false)}
        onError={onError}
        onToast={onToast}
        onDiscoverSkillsChange={onDiscoverSkillsChange}
        onEndpointsChange={setEndpoints}
        onReposChange={setRepos}
        startupRefreshSettings={startupRefreshSettings}
        onStartupRefreshSettingsChange={onStartupRefreshSettingsChange}
      />

      {selectedHubEndpoint && (
        <UploadToHubDialog
          open={uploadDialogOpen}
          hubEndpointId={selectedHubEndpoint.id}
          hubEndpointName={selectedHubEndpoint.name}
          hubState={hubState}
          skillRecords={installedRecords}
          enabledHubEndpoints={enabledHubs}
          onClose={() => setUploadDialogOpen(false)}
          onDiscoverSkillsChange={onDiscoverSkillsChange}
          onPendingUpdatesChange={onPendingUpdatesChange}
          onRefreshHubState={refreshHubState}
          onToast={onToast}
          onError={onError}
        />
      )}
    </section>
  );
}

export default SkillHubPage;
