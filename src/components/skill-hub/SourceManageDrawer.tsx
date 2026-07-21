import { useCallback, useEffect, useRef, useState } from 'react';
import {
  addIflytekSkillHubEndpoint,
  addSkillHubEndpoint,
  addSkillRepo,
  getSkillRepos,
  listGitlabCredentials,
  listIflytekSkillHubEndpoints,
  listSkillHubEndpoints,
  previewAddSkillRepo,
  removeGitlabCredential,
  removeIflytekSkillHubEndpoint,
  removeSkillHubEndpoint,
  removeSkillRepo,
  setIflytekSkillHubEndpointEnabled,
  setSkillHubEndpointEnabled,
  setSkillRepoEnabled,
  setStartupRefreshSettings,
  updateGitlabCredential,
  validateGitlabPat,
} from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import {
  copyTextToClipboard,
  formatHubSourceConfig,
  formatIflytekHubSourceConfig,
  formatRepoSourceConfig,
} from '../../utils/sourceConfigClipboard';
import type {
  DiscoverableSkill,
  IflytekSkillHubEndpoint,
  SkillHubEndpoint,
  SkillRepo,
  StartupRefreshSettings,
} from '../../model/types';
import GitLabPatDialog from './GitLabPatDialog';
import KeysManageDialog from './KeysManageDialog';
import { useModalFocus } from '../../hooks/useModalFocus';

export interface SourceManageDrawerProps {
  open: boolean;
  onClose: () => void;
  onError?: (error: unknown) => void;
  onToast?: (message: string) => void;
  onDiscoverSkillsChange?: (skills: DiscoverableSkill[]) => void;
  onEndpointsChange?: (endpoints: SkillHubEndpoint[]) => void;
  onIflytekEndpointsChange?: (endpoints: IflytekSkillHubEndpoint[]) => void;
  onReposChange?: (repos: SkillRepo[]) => void;
  startupRefreshSettings: StartupRefreshSettings;
  onStartupRefreshSettingsChange?: (settings: StartupRefreshSettings) => void;
}

type AddSourceTab = 'skillsSync' | 'iflytek' | 'github' | 'gitlab';
type ListFilterTab = 'all' | 'skillsSync' | 'iflytek' | 'github' | 'gitlab';

interface PatDialogState {
  host: string;
  url: string;
  projectPath: string;
  mode: 'add' | 'authenticate' | 'update';
}

type DeleteTarget =
  | { kind: 'hub'; endpoint: SkillHubEndpoint }
  | { kind: 'iflytek'; endpoint: IflytekSkillHubEndpoint }
  | { kind: 'repo'; repo: SkillRepo };

const STARTUP_REFRESH_OPTIONS = [
  ['skillHub', 'Skills Sync Hub'],
  ['iflytekSkillHub', 'iFlytek Skill Hub'],
  ['github', 'GitHub'],
  ['gitlab', 'GitLab'],
] as const;

const ADD_SOURCE_TABS: { id: AddSourceTab; label: string }[] = [
  { id: 'skillsSync', label: 'Skills Sync' },
  { id: 'iflytek', label: 'iFlytek' },
  { id: 'github', label: 'GitHub' },
  { id: 'gitlab', label: 'GitLab' },
];

const LIST_FILTER_TABS: { id: ListFilterTab; label: string }[] = [
  { id: 'all', label: '全部' },
  { id: 'skillsSync', label: 'Skills Sync' },
  { id: 'iflytek', label: 'iFlytek' },
  { id: 'github', label: 'GitHub' },
  { id: 'gitlab', label: 'GitLab' },
];

function repoShortPath(repo: SkillRepo): string {
  if (repo.provider === 'gitlab') {
    return `${repo.host}/${repo.projectPath}`;
  }
  return repo.projectPath || `${repo.owner}/${repo.name}`;
}

function repoProviderName(repo: SkillRepo): string {
  return repo.provider === 'gitlab' ? 'GitLab' : 'GitHub';
}

function repoItemKey(repo: SkillRepo): string {
  return `${repo.host}/${repo.projectPath || `${repo.owner}/${repo.name}`}`;
}

function deleteTargetLabel(target: DeleteTarget): string {
  if (target.kind === 'hub' || target.kind === 'iflytek') return target.endpoint.name;
  return repoShortPath(target.repo);
}

function SourceManageDrawer(props: SourceManageDrawerProps) {
  const {
    open,
    onClose,
    onError,
    onToast,
    onDiscoverSkillsChange,
    onEndpointsChange,
    onIflytekEndpointsChange,
    onReposChange,
    startupRefreshSettings,
    onStartupRefreshSettingsChange,
  } = props;
  const [endpoints, setEndpoints] = useState<SkillHubEndpoint[]>([]);
  const [iflytekEndpoints, setIflytekEndpoints] = useState<IflytekSkillHubEndpoint[]>([]);
  const [repos, setRepos] = useState<SkillRepo[]>([]);
  const [configuredHosts, setConfiguredHosts] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [keysDialogOpen, setKeysDialogOpen] = useState(false);
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [addTab, setAddTab] = useState<AddSourceTab>('skillsSync');
  const [listFilter, setListFilter] = useState<ListFilterTab>('all');
  const [patDialog, setPatDialog] = useState<PatDialogState | null>(null);
  const [togglingKey, setTogglingKey] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [localStartupRefresh, setLocalStartupRefresh] = useState(startupRefreshSettings);
  const [savingStartupRefresh, setSavingStartupRefresh] = useState(false);

  const [hubName, setHubName] = useState('');
  const [hubBaseUrl, setHubBaseUrl] = useState('');
  const [repoUrl, setRepoUrl] = useState('');
  const drawerRef = useRef<HTMLDivElement>(null);
  const addModalRef = useRef<HTMLDivElement>(null);
  const deleteModalRef = useRef<HTMLDivElement>(null);

  useModalFocus({ open, containerRef: drawerRef, escapeEnabled: false });
  useModalFocus({ open: addModalOpen, containerRef: addModalRef, escapeEnabled: false });
  useModalFocus({ open: deleteTarget !== null, containerRef: deleteModalRef, escapeEnabled: false });

  useEffect(() => {
    setLocalStartupRefresh(startupRefreshSettings);
  }, [startupRefreshSettings]);

  const handleStartupRefreshToggle = async (
    key: keyof StartupRefreshSettings,
    checked: boolean,
  ) => {
    const previous = localStartupRefresh;
    const next = { ...previous, [key]: checked };
    setLocalStartupRefresh(next);
    setSavingStartupRefresh(true);
    try {
      const saved = await setStartupRefreshSettings(next);
      setLocalStartupRefresh(saved);
      onStartupRefreshSettingsChange?.(saved);
    } catch (err) {
      setLocalStartupRefresh(previous);
      onError?.(errorMessage(err));
    } finally {
      setSavingStartupRefresh(false);
    }
  };

  const loadConfiguredHosts = useCallback(async () => {
    try {
      const hosts = await listGitlabCredentials();
      setConfiguredHosts(hosts);
    } catch (err) {
      onError?.(errorMessage(err));
    }
  }, [onError]);

  const loadSources = useCallback(async () => {
    setLoading(true);
    try {
      const [endpointList, iflytekList, repoList] = await Promise.all([
        listSkillHubEndpoints(),
        listIflytekSkillHubEndpoints(),
        getSkillRepos(),
      ]);
      const nextEndpoints = endpointList ?? [];
      const nextIflytek = iflytekList ?? [];
      const nextRepos = repoList ?? [];
      setEndpoints(nextEndpoints);
      setIflytekEndpoints(nextIflytek);
      setRepos(nextRepos);
      onEndpointsChange?.(nextEndpoints);
      onIflytekEndpointsChange?.(nextIflytek);
      onReposChange?.(nextRepos);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [onError, onEndpointsChange, onIflytekEndpointsChange, onReposChange]);

  useEffect(() => {
    if (!open) return;
    void loadSources();
    void loadConfiguredHosts();
  }, [open, loadSources, loadConfiguredHosts]);

  useEffect(() => {
    if (!keysDialogOpen) return;
    void loadConfiguredHosts();
  }, [keysDialogOpen, loadConfiguredHosts]);

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || patDialog || keysDialogOpen) return;
      if (deleteTarget) {
        if (!deleting) {
          setDeleteTarget(null);
          setDeleteError(null);
        }
        return;
      }
      if (addModalOpen) {
        if (!adding) {
          setAddModalOpen(false);
          setAddError(null);
          resetAddForm();
        }
        return;
      }
      onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose, patDialog, keysDialogOpen, addModalOpen, adding, deleteTarget, deleting]);

  const resetAddForm = () => {
    setHubName('');
    setHubBaseUrl('');
    setRepoUrl('');
    setAddTab('skillsSync');
  };

  const completeAdd = () => {
    setPatDialog(null);
    setAddModalOpen(false);
    setAddError(null);
    resetAddForm();
    onToast?.('来源已添加');
    onClose();
  };

  const finishAddRepo = async (addUrl: string, pat?: string) => {
    const result = await addSkillRepo(addUrl, undefined, pat);
    setRepos(result.repos);
    onDiscoverSkillsChange?.(result.discoverSkills);
    onReposChange?.(result.repos);
    await loadConfiguredHosts();
  };

  const handleAddHub = async () => {
    const name = hubName.trim();
    const baseUrl = hubBaseUrl.trim();
    if (!name || !baseUrl || adding) return;

    setAdding(true);
    setAddError(null);
    try {
      const result = await addSkillHubEndpoint(name, baseUrl);
      setEndpoints(result.endpoints);
      onEndpointsChange?.(result.endpoints);
      onDiscoverSkillsChange?.(result.discoverSkills);
      completeAdd();
    } catch (err) {
      setAddError(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handleAddIflytek = async () => {
    const name = hubName.trim();
    const baseUrl = hubBaseUrl.trim();
    if (!name || !baseUrl || adding) return;

    setAdding(true);
    setAddError(null);
    try {
      const result = await addIflytekSkillHubEndpoint(name, baseUrl);
      const next = result.iflytekSkillHubEndpoints ?? [];
      setIflytekEndpoints(next);
      onIflytekEndpointsChange?.(next);
      onDiscoverSkillsChange?.(result.discoverSkills);
      completeAdd();
    } catch (err) {
      setAddError(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handleAddRepo = async () => {
    const value = repoUrl.trim();
    if (!value || adding) return;

    setAdding(true);
    setAddError(null);
    try {
      const preview = await previewAddSkillRepo(value);
      if (preview.error) {
        setAddError(preview.error.message);
        return;
      }
      if (preview.needsPat && preview.host) {
        setPatDialog({
          host: preview.host,
          url: value,
          projectPath: preview.projectPath ?? value.replace(/^https?:\/\//, '').replace(/\/$/, ''),
          mode: 'add',
        });
        return;
      }
      await finishAddRepo(value);
      completeAdd();
    } catch (err) {
      setAddError(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handlePatSubmit = async (pat: string) => {
    if (!patDialog) return;

    const { host, url: addUrl, mode } = patDialog;
    await validateGitlabPat(host, pat);

    if (mode === 'authenticate' || mode === 'update') {
      await updateGitlabCredential(host, pat);
      await loadConfiguredHosts();
      return;
    }

    setAdding(true);
    try {
      await updateGitlabCredential(host, pat);
      await loadConfiguredHosts();
      await finishAddRepo(addUrl, pat);
      completeAdd();
    } finally {
      setAdding(false);
    }
  };

  const handleConfirmDelete = async () => {
    if (!deleteTarget || deleting) return;
    setDeleteError(null);
    setDeleting(true);
    try {
      if (deleteTarget.kind === 'hub') {
        const result = await removeSkillHubEndpoint(deleteTarget.endpoint.id);
        setEndpoints(result.endpoints);
        onEndpointsChange?.(result.endpoints);
        onDiscoverSkillsChange?.(result.discoverSkills);
      } else if (deleteTarget.kind === 'iflytek') {
        const result = await removeIflytekSkillHubEndpoint(deleteTarget.endpoint.id);
        const next = result.iflytekSkillHubEndpoints ?? [];
        setIflytekEndpoints(next);
        onIflytekEndpointsChange?.(next);
        onDiscoverSkillsChange?.(result.discoverSkills);
      } else {
        const { repo } = deleteTarget;
        const result = await removeSkillRepo(repo.host, repo.projectPath);
        setRepos(result.repos);
        onDiscoverSkillsChange?.(result.discoverSkills);
        onReposChange?.(result.repos);
      }
      setDeleteTarget(null);
      onToast?.('来源已删除');
    } catch (err) {
      setDeleteError(errorMessage(err));
    } finally {
      setDeleting(false);
    }
  };

  const handleToggleHub = async (endpoint: SkillHubEndpoint, enabled: boolean) => {
    const key = `hub:${endpoint.id}`;
    if (togglingKey) return;
    setTogglingKey(key);
    try {
      const result = await setSkillHubEndpointEnabled(endpoint.id, enabled);
      setEndpoints(result.endpoints);
      onEndpointsChange?.(result.endpoints);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setTogglingKey(null);
    }
  };

  const handleToggleIflytek = async (endpoint: IflytekSkillHubEndpoint, enabled: boolean) => {
    const key = `iflytek:${endpoint.id}`;
    if (togglingKey) return;
    setTogglingKey(key);
    try {
      const result = await setIflytekSkillHubEndpointEnabled(endpoint.id, enabled);
      const next = result.iflytekSkillHubEndpoints ?? [];
      setIflytekEndpoints(next);
      onIflytekEndpointsChange?.(next);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setTogglingKey(null);
    }
  };

  const handleToggleRepo = async (repo: SkillRepo, enabled: boolean) => {
    const key = repoItemKey(repo);
    if (togglingKey) return;
    setTogglingKey(key);
    try {
      const result = await setSkillRepoEnabled(repo.host, repo.projectPath, enabled);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
      onReposChange?.(result.repos);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setTogglingKey(null);
    }
  };

  const handleRemoveCredential = async (host: string) => {
    await removeGitlabCredential(host);
    await loadConfiguredHosts();
  };

  const handleAuthenticateCredential = (host: string) => {
    setPatDialog({ host, url: '', projectPath: host, mode: 'authenticate' });
  };

  const handleUpdateCredential = (host: string) => {
    setPatDialog({ host, url: '', projectPath: host, mode: 'update' });
  };

  const handleCopyText = async (text: string, label: string) => {
    try {
      await copyTextToClipboard(text);
      onToast?.(`已复制${label}配置`);
    } catch (err) {
      onError?.(errorMessage(err));
    }
  };

  const handleCopyHub = (endpoint: SkillHubEndpoint) => {
    void handleCopyText(formatHubSourceConfig(endpoint), endpoint.name);
  };

  const handleCopyIflytek = (endpoint: IflytekSkillHubEndpoint) => {
    void handleCopyText(formatIflytekHubSourceConfig(endpoint), endpoint.name);
  };

  const handleCopyRepo = (repo: SkillRepo) => {
    void handleCopyText(formatRepoSourceConfig(repo), repoShortPath(repo));
  };

  const handleAddSubmit = () => {
    if (addTab === 'skillsSync') void handleAddHub();
    else if (addTab === 'iflytek') void handleAddIflytek();
    else void handleAddRepo();
  };

  if (!open) return null;

  const showSkillsSync = listFilter === 'all' || listFilter === 'skillsSync';
  const showIflytek = listFilter === 'all' || listFilter === 'iflytek';
  const showGithub = listFilter === 'all' || listFilter === 'github';
  const showGitlab = listFilter === 'all' || listFilter === 'gitlab';

  const filteredEndpoints = showSkillsSync ? endpoints : [];
  const filteredIflytek = showIflytek ? iflytekEndpoints : [];
  const filteredRepos = repos.filter((repo) => {
    if (repo.provider === 'github') return showGithub;
    if (repo.provider === 'gitlab') return showGitlab;
    return listFilter === 'all';
  });

  const hasAnySource =
    endpoints.length > 0 || iflytekEndpoints.length > 0 || repos.length > 0;
  const hasFilteredSource =
    filteredEndpoints.length > 0 || filteredIflytek.length > 0 || filteredRepos.length > 0;

  const isHubForm = addTab === 'skillsSync' || addTab === 'iflytek';
  const canSubmitAdd = isHubForm
    ? hubName.trim().length > 0 && hubBaseUrl.trim().length > 0
    : repoUrl.trim().length > 0;

  return (
    <>
      <div
        className="overlay drawer-overlay open"
        role="dialog"
        aria-modal="true"
        aria-label="来源管理"
        onClick={() => {
          if (!adding && !patDialog && !keysDialogOpen && !addModalOpen && !deleteTarget) onClose();
        }}
      >
        <div
          ref={drawerRef}
          className="drawer source-manage-drawer"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="drawer-header-row">
            <div>
              <h2>来源管理</h2>
            </div>
            <div className="drawer-header-actions">
              <button type="button" className="btn-keys-link" onClick={() => setKeysDialogOpen(true)}>
                密钥管理
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => {
                  setAddError(null);
                  setAddModalOpen(true);
                }}
              >
                添加来源
              </button>
            </div>
          </div>

          <section className="startup-refresh-settings" aria-labelledby="startup-refresh-title">
            <div>
              <h3 id="startup-refresh-title">启动自动刷新</h3>
              <p>仅影响应用启动时的后台刷新；手动刷新始终检查所有已启用来源。</p>
            </div>
            <div className="startup-refresh-options">
              {STARTUP_REFRESH_OPTIONS.map(([key, label]) => (
                <label key={key} className="startup-refresh-option">
                  <span>{label}</span>
                  <input
                    type="checkbox"
                    checked={Boolean(localStartupRefresh[key])}
                    disabled={savingStartupRefresh}
                    aria-label={`${label} 启动自动刷新`}
                    onChange={(event) =>
                      void handleStartupRefreshToggle(key, event.target.checked)
                    }
                  />
                </label>
              ))}
            </div>
          </section>

          <div className="text-tabs source-list-filter-tabs" role="tablist" aria-label="来源筛选">
            {LIST_FILTER_TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                role="tab"
                className={`text-tab${listFilter === tab.id ? ' active' : ''}`}
                aria-selected={listFilter === tab.id}
                onClick={() => setListFilter(tab.id)}
              >
                {tab.label}
              </button>
            ))}
          </div>

          {loading ? (
            <p className="drawer-loading">加载中…</p>
          ) : (
            <ul className="repo-list source-list">
              {!hasAnySource ? (
                <li className="repo-empty">暂无来源，请点击「添加来源」。</li>
              ) : !hasFilteredSource ? (
                <li className="repo-empty">当前筛选下暂无来源。</li>
              ) : (
                <>
                  {filteredEndpoints.map((endpoint) => {
                    const key = `hub:${endpoint.id}`;
                    const toggling = togglingKey === key;
                    return (
                      <li
                        key={key}
                        className={`repo-item source-item${endpoint.enabled ? '' : ' repo-item-disabled'}`}
                      >
                        <div className="repo-item-name source-item-title" title={endpoint.name}>
                          {endpoint.name}
                        </div>
                        <label
                          className="repo-switch source-item-switch"
                          title={endpoint.enabled ? '停用此来源' : '启用此来源'}
                        >
                          <input
                            type="checkbox"
                            checked={endpoint.enabled}
                            disabled={toggling}
                            onChange={(e) => void handleToggleHub(endpoint, e.target.checked)}
                            aria-label={`${endpoint.enabled ? '停用' : '启用'} ${endpoint.name}`}
                          />
                          <span className="repo-switch-slider" aria-hidden />
                        </label>
                        <span className="repo-provider-tag repo-provider-hub" aria-hidden>
                          Skills Sync
                        </span>
                        <div className="source-item-secondary">
                          <span className="source-item-url" title={endpoint.baseUrl}>
                            {endpoint.baseUrl}
                          </span>
                          {!endpoint.enabled && <span className="badge-disabled">已停用</span>}
                        </div>
                        <div className="source-item-actions">
                          <button
                            type="button"
                            className="btn-repo-copy"
                            onClick={() => handleCopyHub(endpoint)}
                            aria-label={`复制 ${endpoint.name} 配置`}
                          >
                            复制
                          </button>
                          <button
                            type="button"
                            className="btn-repo-remove"
                            onClick={() => {
                              setDeleteError(null);
                              setDeleteTarget({ kind: 'hub', endpoint });
                            }}
                            aria-label={`删除 ${endpoint.name}`}
                          >
                            删除
                          </button>
                        </div>
                      </li>
                    );
                  })}
                  {filteredIflytek.map((endpoint) => {
                    const key = `iflytek:${endpoint.id}`;
                    const toggling = togglingKey === key;
                    return (
                      <li
                        key={key}
                        className={`repo-item source-item${endpoint.enabled ? '' : ' repo-item-disabled'}`}
                      >
                        <div className="repo-item-name source-item-title" title={endpoint.name}>
                          {endpoint.name}
                        </div>
                        <label
                          className="repo-switch source-item-switch"
                          title={endpoint.enabled ? '停用此来源' : '启用此来源'}
                        >
                          <input
                            type="checkbox"
                            checked={endpoint.enabled}
                            disabled={toggling}
                            onChange={(e) => void handleToggleIflytek(endpoint, e.target.checked)}
                            aria-label={`${endpoint.enabled ? '停用' : '启用'} ${endpoint.name}`}
                          />
                          <span className="repo-switch-slider" aria-hidden />
                        </label>
                        <span className="repo-provider-tag repo-provider-iflytek" aria-hidden>
                          iFlytek
                        </span>
                        <div className="source-item-secondary">
                          <span className="source-item-url" title={endpoint.baseUrl}>
                            {endpoint.baseUrl}
                          </span>
                          {!endpoint.enabled && <span className="badge-disabled">已停用</span>}
                        </div>
                        <div className="source-item-actions">
                          <button
                            type="button"
                            className="btn-repo-copy"
                            onClick={() => handleCopyIflytek(endpoint)}
                            aria-label={`复制 ${endpoint.name} 配置`}
                          >
                            复制
                          </button>
                          <button
                            type="button"
                            className="btn-repo-remove"
                            onClick={() => {
                              setDeleteError(null);
                              setDeleteTarget({ kind: 'iflytek', endpoint });
                            }}
                            aria-label={`删除 ${endpoint.name}`}
                          >
                            删除
                          </button>
                        </div>
                      </li>
                    );
                  })}
                  {filteredRepos.map((repo) => {
                    const path = repoShortPath(repo);
                    const authed =
                      repo.provider === 'gitlab' && configuredHosts.includes(repo.host);
                    const itemKey = repoItemKey(repo);
                    const toggling = togglingKey === itemKey;
                    return (
                      <li
                        key={itemKey}
                        className={`repo-item source-item${repo.enabled ? '' : ' repo-item-disabled'}`}
                      >
                        <div className="repo-item-name source-item-title" title={path}>
                          {path}
                        </div>
                        <label
                          className="repo-switch source-item-switch"
                          title={repo.enabled ? '停用此来源' : '启用此来源'}
                        >
                          <input
                            type="checkbox"
                            checked={repo.enabled}
                            disabled={toggling}
                            onChange={(e) => void handleToggleRepo(repo, e.target.checked)}
                            aria-label={`${repo.enabled ? '停用' : '启用'} ${path}`}
                          />
                          <span className="repo-switch-slider" aria-hidden />
                        </label>
                        <span
                          className={`repo-provider-tag repo-provider-${repo.provider}`}
                          aria-hidden
                        >
                          {repoProviderName(repo)}
                        </span>
                        <div className="source-item-secondary">
                          <div className="repo-item-meta-row source-item-meta-row">
                            <span className="repo-provider-tag repo-provider-branch">{repo.branch}</span>
                            {authed && <span className="badge-auth">已认证</span>}
                            {!repo.enabled && <span className="badge-disabled">已停用</span>}
                          </div>
                        </div>
                        <div className="source-item-actions">
                          <button
                            type="button"
                            className="btn-repo-copy"
                            onClick={() => handleCopyRepo(repo)}
                            aria-label={`复制 ${path} 配置`}
                          >
                            复制
                          </button>
                          <button
                            type="button"
                            className="btn-repo-remove"
                            onClick={() => {
                              setDeleteError(null);
                              setDeleteTarget({ kind: 'repo', repo });
                            }}
                            aria-label={`删除 ${path}`}
                          >
                            删除
                          </button>
                        </div>
                      </li>
                    );
                  })}
                </>
              )}
            </ul>
          )}

          <div className="drawer-footer">
            <button type="button" onClick={onClose}>
              关闭
            </button>
          </div>
        </div>
      </div>

      {addModalOpen && (
        <div
          className="overlay open add-source-overlay"
          role="dialog"
          aria-modal="true"
          aria-label="添加来源"
          onClick={() => {
            if (!adding) {
              setAddModalOpen(false);
              setAddError(null);
              resetAddForm();
            }
          }}
        >
          <div
            ref={addModalRef}
            className="modal add-source-modal"
            onClick={(e) => e.stopPropagation()}
          >
            <h3>添加来源</h3>
            <div className="text-tabs add-source-tabs" role="tablist">
              {ADD_SOURCE_TABS.map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  role="tab"
                  className={`text-tab${addTab === tab.id ? ' active' : ''}`}
                  aria-selected={addTab === tab.id}
                  disabled={adding}
                  onClick={() => {
                    setAddError(null);
                    setAddTab(tab.id);
                  }}
                >
                  {tab.label}
                </button>
              ))}
            </div>

            {isHubForm ? (
              <div className="add-source-form">
                <label>
                  名称
                  <input
                    type="text"
                    value={hubName}
                    onChange={(e) => {
                      setHubName(e.target.value);
                      if (addError) setAddError(null);
                    }}
                    placeholder={
                      addTab === 'iflytek' ? '公司 iFlytek Skill Hub' : '公司 Skills Sync Hub'
                    }
                    disabled={adding}
                  />
                </label>
                <label>
                  Base URL
                  <input
                    type="text"
                    value={hubBaseUrl}
                    onChange={(e) => {
                      setHubBaseUrl(e.target.value);
                      if (addError) setAddError(null);
                    }}
                    placeholder="https://hub.example.com"
                    disabled={adding}
                  />
                </label>
              </div>
            ) : (
              <div className="add-source-form">
                <label>
                  仓库链接
                  <input
                    type="text"
                    value={repoUrl}
                    onChange={(e) => {
                      setRepoUrl(e.target.value);
                      if (addError) setAddError(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') void handleAddRepo();
                    }}
                    placeholder={
                      addTab === 'github'
                        ? 'https://github.com/owner/repo'
                        : 'https://gitlab.example.com/group/project'
                    }
                    disabled={adding}
                  />
                </label>
              </div>
            )}

            <div className="add-source-error-slot">
              {addError && (
                <p className="modal-error show" role="alert">
                  {addError}
                </p>
              )}
            </div>

            <div className="modal-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => {
                  setAddModalOpen(false);
                  setAddError(null);
                  resetAddForm();
                }}
                disabled={adding}
              >
                取消
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={handleAddSubmit}
                disabled={adding || !canSubmitAdd}
              >
                {adding ? '添加中…' : '添加'}
              </button>
            </div>
          </div>
        </div>
      )}

      <KeysManageDialog
        open={keysDialogOpen}
        repos={repos}
        configuredHosts={configuredHosts}
        nestedDialogOpen={patDialog !== null}
        onClose={() => setKeysDialogOpen(false)}
        onAuthenticate={handleAuthenticateCredential}
        onUpdate={handleUpdateCredential}
        onRemove={handleRemoveCredential}
      />

      <GitLabPatDialog
        open={patDialog !== null}
        host={patDialog?.host ?? ''}
        description={
          patDialog?.mode === 'update' || patDialog?.mode === 'authenticate'
            ? (patDialog?.host ?? '')
            : (patDialog?.projectPath ?? '')
        }
        mode={patDialog?.mode ?? 'add'}
        onClose={() => setPatDialog(null)}
        onSubmit={handlePatSubmit}
        submitLabel={patDialog?.mode === 'add' ? '验证并添加' : '验证并保存'}
      />

      {deleteTarget && (
        <div
          className="modal-overlay open source-delete-overlay"
          role="dialog"
          aria-modal="true"
          aria-labelledby="sourceDeleteTitle"
          onClick={() => {
            if (!deleting) {
              setDeleteTarget(null);
              setDeleteError(null);
            }
          }}
        >
          <div ref={deleteModalRef} className="modal" onClick={(event) => event.stopPropagation()}>
            <h3 id="sourceDeleteTitle">删除来源</h3>
            <p>
              确认删除 <strong>{deleteTargetLabel(deleteTarget)}</strong>？删除后可以重新添加。
            </p>
            <div className="source-delete-error-slot">
              {deleteError && (
                <p className="modal-error show" role="alert">
                  {deleteError}
                </p>
              )}
            </div>
            <div className="modal-actions">
              <button
                type="button"
                className="cancel"
                disabled={deleting}
                onClick={() => {
                  setDeleteTarget(null);
                  setDeleteError(null);
                }}
              >
                取消
              </button>
              <button
                type="button"
                className="danger-button"
                disabled={deleting}
                onClick={() => void handleConfirmDelete()}
              >
                {deleting ? '删除中…' : '确认删除'}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export default SourceManageDrawer;
