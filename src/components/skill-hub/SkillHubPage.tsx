import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  checkSkillUpdates,
  discoverSkills,
  installHubSkill,
  scanMainLibrary,
  updateAllSkills,
  updateSkill,
} from '../../api/skillHub';
import type {
  DiscoverableSkill,
  SkillHubLocalState,
  SkillRecord,
  SkillUpdateInfo,
} from '../../model/types';
import { errorMessage } from '../../utils/errorMessage';
import InstallConfirmDialog from './InstallConfirmDialog';
import RepoDrawer from './RepoDrawer';
import SkillCard from './SkillCard';
import SmartPasteBar from './SmartPasteBar';

type HubTab = 'installed' | 'discover';
type InstalledChip = 'all' | 'updates';
type SourceFilter = 'all' | 'manual' | 'github' | 'gitlab' | 'skillssh';

export interface SkillHubPageProps {
  mainSkillsDir: string | null;
  hubState: SkillHubLocalState;
  discoverSkills: DiscoverableSkill[];
  pendingUpdates: SkillUpdateInfo[];
  skillRecords?: Record<string, SkillRecord>;
  onHubStateChange: (state: SkillHubLocalState) => void;
  onDiscoverSkillsChange: (skills: DiscoverableSkill[]) => void;
  onPendingUpdatesChange: (updates: SkillUpdateInfo[]) => void;
  onDeleteMainSkill: (skillDirName: string) => void;
  onSetMainSkillsDir: () => void;
  onRefreshHub?: () => Promise<void>;
  onToast?: (message: string) => void;
  onError?: (error: unknown) => void;
}

function sourceLabel(source: string, record?: Pick<SkillRecord, 'repoHost'>): string {
  if (source === 'github') return 'GitHub';
  if (source === 'skillssh') return 'skills.sh';
  if (source === 'gitlab') {
    return record?.repoHost ? `GitLab · ${record.repoHost}` : 'GitLab';
  }
  return '本地导入';
}

function getInstalledSource(
  dirName: string,
  skillRecords?: Record<string, SkillRecord>,
): string {
  const record = skillRecords?.[dirName];
  if (!record) return 'manual';
  if (record.source === 'skillssh') return 'skillssh';
  if (record.source === 'github') return 'github';
  if (record.source === 'gitlab') return 'gitlab';
  return 'manual';
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
    onHubStateChange,
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
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all');
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [repoDrawerOpen, setRepoDrawerOpen] = useState(false);
  const [installDialogOpen, setInstallDialogOpen] = useState(false);
  const [pendingInstall, setPendingInstall] = useState<DiscoverableSkill[]>([]);
  const [installing, setInstalling] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updatingAll, setUpdatingAll] = useState(false);
  const [refreshingDiscover, setRefreshingDiscover] = useState(false);
  const [hubInstalling, setHubInstalling] = useState(false);

  const discoverInFlight = useRef(false);
  const checkInFlight = useRef(false);

  const pendingUpdateSet = useMemo(
    () => new Set(pendingUpdates.map((u) => u.dirName)),
    [pendingUpdates],
  );

  const refreshHubState = useCallback(async () => {
    if (onRefreshHub) {
      await onRefreshHub();
      return;
    }
    const next = await scanMainLibrary();
    onHubStateChange(next);
  }, [onRefreshHub, onHubStateChange]);

  useEffect(() => {
    void refreshHubState().catch((err) => {
      onError?.(errorMessage(err));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (tab === 'discover' && sourceFilter === 'manual') {
      setSourceFilter('all');
    }
  }, [tab, sourceFilter]);

  const handleCheckUpdates = async () => {
    if (checkInFlight.current) return;
    checkInFlight.current = true;
    setCheckingUpdates(true);
    try {
      const updates = await checkSkillUpdates();
      onPendingUpdatesChange(updates);
      const next = await scanMainLibrary();
      onHubStateChange(next);
      onToast?.(updates.length > 0 ? `发现 ${updates.length} 个更新` : '暂无更新');
    } catch (err) {
      onError?.(errorMessage(err));
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
    if (discoverInFlight.current) return;
    discoverInFlight.current = true;
    setRefreshingDiscover(true);
    try {
      const result = await discoverSkills();
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
      onError?.(errorMessage(err));
    } finally {
      discoverInFlight.current = false;
      setRefreshingDiscover(false);
    }
  };

  const handleUpdateSkill = async (dirName: string) => {
    try {
      await updateSkill(dirName);
      onPendingUpdatesChange(pendingUpdates.filter((u) => u.dirName !== dirName));
      await refreshHubState();
      onToast?.('已更新');
    } catch (err) {
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
      await refreshHubState().catch(() => {});
      onError?.(errorMessage(err));
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
    try {
      const keys = pendingInstall.map((s) => s.key);
      for (const skill of pendingInstall) {
        await installHubSkill(skill);
      }
      onDiscoverSkillsChange(discoverList.filter((s) => !keys.includes(s.key)));
      setSelectedKeys((prev) => {
        const next = new Set(prev);
        keys.forEach((k) => next.delete(k));
        return next;
      });
      await refreshHubState();
      onToast?.(
        keys.length > 1 ? `已安装 ${keys.length} 个 Skill 到主库` : '已安装到主库',
      );
    } catch (err) {
      await refreshHubState().catch(() => {});
      onError?.(errorMessage(err));
    } finally {
      setInstallDialogOpen(false);
      setPendingInstall([]);
      setInstalling(false);
    }
  };

  const installedRecords = skillRecords ?? hubState.skillRecords;

  const filteredInstalled = useMemo(() => {
    return hubState.skills.filter((skill) => {
      const hasUpdate = pendingUpdateSet.has(skill.dirName);
      if (installedChip === 'updates' && !hasUpdate) return false;

      const source = getInstalledSource(skill.dirName, installedRecords);
      if (sourceFilter !== 'all' && source !== sourceFilter) return false;

      return matchesSearch(search, [
        skill.name,
        skill.description,
        skill.dirName,
        skill.path,
      ]);
    });
  }, [
    hubState.skills,
    installedChip,
    pendingUpdateSet,
    search,
    sourceFilter,
    installedRecords,
  ]);

  const filteredDiscover = useMemo(() => {
    return discoverList.filter((skill) => {
      if (sourceFilter !== 'all' && skill.source !== sourceFilter) return false;
      return matchesSearch(search, [
        skill.name,
        skill.description,
        skill.installDirName,
        skill.directory,
        `${skill.repoOwner}/${skill.repoName}`,
      ]);
    });
  }, [discoverList, search, sourceFilter]);

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
  const installedCount = hubState.skills.length;
  const discoverCount = discoverList.length;
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

  return (
    <section className="skill-hub-page">
      {hubBusy && (
        <div className="hub-busy-overlay" aria-live="polite">
          {hubBusyLabel}
        </div>
      )}
      <div className="hub-body">
        <div className="hub-hero">
          <div className="hub-hero-row">
            <div className="hub-hero-main">
              <div className="hub-title-row">
                <h1>Skill 中心</h1>
                <div className="hub-stat-row">
                  <span className="pill hub-pill">{hubState.validCount} 有效</span>
                  {hubState.invalidCount > 0 && (
                    <span className="pill hub-pill warn">
                      {hubState.invalidCount} 无效
                    </span>
                  )}
                  {pendingCount > 0 && (
                    <span className="pill hub-pill update">{pendingCount} 待更新</span>
                  )}
                </div>
              </div>
              <div className="dir-path-row hub-path-row">
                <div
                  className="dir-path"
                  title="主库路径"
                >
                  {mainSkillsDir ?? '未设置主库目录'}
                </div>
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
            </div>
            <div className="hub-actions">
              <button
                type="button"
                onClick={() => void handleCheckUpdates()}
                disabled={checkBusy || updatingAll}
              >
                {checkBusy ? '检查更新…' : '检查更新'}
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => void handleUpdateAll()}
                disabled={pendingCount === 0 || updatingAll || checkBusy}
              >
                {updatingAll ? '更新中…' : `全部更新 (${pendingCount})`}
              </button>
              <button type="button" onClick={() => setRepoDrawerOpen(true)}>
                仓库管理
              </button>
            </div>
          </div>
        </div>

        <SmartPasteBar
          onInstall={handleInstallSkill}
          onError={onError}
        />

        <div className="hub-toolbar">
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
              已安装 ({installedCount})
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
              可安装 ({discoverCount})
            </button>
          </div>
          {tab === 'installed' && (
            <div className="chip-group visible" id="installedChips">
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
          <div className="filter-inline">
            <select
              value={sourceFilter}
              onChange={(e) => setSourceFilter(e.target.value as SourceFilter)}
              aria-label="来源筛选"
            >
              <option value="all">全部来源</option>
              {tab === 'installed' && <option value="manual">本地导入</option>}
              <option value="github">GitHub</option>
              <option value="gitlab">GitLab</option>
              <option value="skillssh">skills.sh</option>
            </select>
          </div>
        </div>

        {tab === 'discover' && (
          <div className="tab-actions visible">
            <button
              type="button"
              onClick={() => void handleRefreshDiscover()}
              disabled={discoverBusy}
            >
              {refreshingDiscover ? '拉取中…' : '刷新列表'}
            </button>
          </div>
        )}

        {showBatchBar && (
          <div className="batch-bar visible">
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
                  key={skill.dirName}
                  skill={skill}
                  mode="installed"
                  hasUpdate={pendingUpdateSet.has(skill.dirName)}
                  sourceLabel={sourceLabel(
                    getInstalledSource(skill.dirName, installedRecords),
                    installedRecords?.[skill.dirName],
                  )}
                  onUpdate={() => void handleUpdateSkill(skill.dirName)}
                  onDelete={() => onDeleteMainSkill(skill.dirName)}
                />
              ))
            ) : (
              <div className="empty-hint">无匹配结果</div>
            )
          ) : filteredDiscover.length > 0 ? (
            filteredDiscover.map((skill) => (
              <SkillCard
                key={skill.key}
                skill={skill}
                mode="discover"
                selected={selectedKeys.has(skill.key)}
                sourceLabel={sourceLabel(skill.source, skill)}
                onSelect={(selected) => toggleSelection(skill.key, selected)}
                onInstall={() => openInstallDialog([skill])}
              />
            ))
          ) : (
            <div className="empty-hint">
              {discoverList.length === 0 ? '暂无列表，点击「刷新列表」' : '无匹配结果'}
            </div>
          )}
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

      <RepoDrawer
        open={repoDrawerOpen}
        onClose={() => setRepoDrawerOpen(false)}
        onError={onError}
        onDiscoverSkillsChange={onDiscoverSkillsChange}
      />
    </section>
  );
}

export default SkillHubPage;
