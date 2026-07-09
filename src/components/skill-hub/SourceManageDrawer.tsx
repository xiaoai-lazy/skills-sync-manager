import { useCallback, useEffect, useState } from 'react';
import {
  addSkillHubEndpoint,
  addSkillRepo,
  getSkillRepos,
  listGitlabCredentials,
  listSkillHubEndpoints,
  previewAddSkillRepo,
  removeGitlabCredential,
  removeSkillHubEndpoint,
  removeSkillRepo,
  setSkillHubEndpointEnabled,
  setSkillRepoEnabled,
  updateGitlabCredential,
  validateGitlabPat,
} from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import {
  copyTextToClipboard,
  formatHubSourceConfig,
  formatRepoSourceConfig,
} from '../../utils/sourceConfigClipboard';
import type { DiscoverableSkill, SkillHubEndpoint, SkillRepo } from '../../model/types';
import GitLabPatDialog from './GitLabPatDialog';
import KeysManageDialog from './KeysManageDialog';

export interface SourceManageDrawerProps {
  open: boolean;
  onClose: () => void;
  onError?: (error: unknown) => void;
  onToast?: (message: string) => void;
  onDiscoverSkillsChange?: (skills: DiscoverableSkill[]) => void;
  onEndpointsChange?: (endpoints: SkillHubEndpoint[]) => void;
  onReposChange?: (repos: SkillRepo[]) => void;
}

type AddSourceTab = 'hub' | 'github' | 'gitlab';

interface PatDialogState {
  host: string;
  url: string;
  projectPath: string;
  mode: 'add' | 'update';
}

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

function SourceManageDrawer(props: SourceManageDrawerProps) {
  const { open, onClose, onError, onToast, onDiscoverSkillsChange, onEndpointsChange, onReposChange } = props;
  const [endpoints, setEndpoints] = useState<SkillHubEndpoint[]>([]);
  const [repos, setRepos] = useState<SkillRepo[]>([]);
  const [configuredHosts, setConfiguredHosts] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);
  const [keysDialogOpen, setKeysDialogOpen] = useState(false);
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [addTab, setAddTab] = useState<AddSourceTab>('hub');
  const [patDialog, setPatDialog] = useState<PatDialogState | null>(null);
  const [togglingKey, setTogglingKey] = useState<string | null>(null);

  const [hubName, setHubName] = useState('');
  const [hubBaseUrl, setHubBaseUrl] = useState('');
  const [repoUrl, setRepoUrl] = useState('');

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
      const [endpointList, repoList] = await Promise.all([
        listSkillHubEndpoints(),
        getSkillRepos(),
      ]);
      setEndpoints(endpointList);
      setRepos(repoList);
      onEndpointsChange?.(endpointList);
      onReposChange?.(repoList);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [onError, onEndpointsChange, onReposChange]);

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
      if (e.key === 'Escape' && !patDialog && !keysDialogOpen && !addModalOpen) onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose, patDialog, keysDialogOpen, addModalOpen]);

  const resetAddForm = () => {
    setHubName('');
    setHubBaseUrl('');
    setRepoUrl('');
    setAddTab('hub');
  };

  const finishAddRepo = async (addUrl: string, pat?: string) => {
    setAdding(true);
    try {
      const result = await addSkillRepo(addUrl, undefined, pat);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
      onReposChange?.(result.repos);
      setRepoUrl('');
      setAddModalOpen(false);
      resetAddForm();
      await loadConfiguredHosts();
    } catch (err) {
      onError?.(errorMessage(err));
      throw err;
    } finally {
      setAdding(false);
    }
  };

  const handleAddHub = async () => {
    const name = hubName.trim();
    const baseUrl = hubBaseUrl.trim();
    if (!name || !baseUrl || adding) return;

    setAdding(true);
    try {
      const result = await addSkillHubEndpoint(name, baseUrl);
      setEndpoints(result.endpoints);
      onEndpointsChange?.(result.endpoints);
      onDiscoverSkillsChange?.(result.discoverSkills);
      setAddModalOpen(false);
      resetAddForm();
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handleAddRepo = async () => {
    const value = repoUrl.trim();
    if (!value || adding) return;

    setAdding(true);
    try {
      const preview = await previewAddSkillRepo(value);
      if (preview.error) {
        onError?.(preview.error.message);
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
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handlePatSubmit = async (pat: string) => {
    if (!patDialog) return;

    const { host, url: addUrl, mode } = patDialog;
    await validateGitlabPat(host, pat);

    if (mode === 'update') {
      await updateGitlabCredential(host, pat);
      await loadConfiguredHosts();
      return;
    }

    await updateGitlabCredential(host, pat);
    await loadConfiguredHosts();
    void finishAddRepo(addUrl, pat);
  };

  const handleRemoveHub = async (endpoint: SkillHubEndpoint) => {
    try {
      const result = await removeSkillHubEndpoint(endpoint.id);
      setEndpoints(result.endpoints);
      onEndpointsChange?.(result.endpoints);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
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

  const handleRemoveRepo = async (repo: SkillRepo) => {
    try {
      const result = await removeSkillRepo(repo.host, repo.projectPath);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
      onReposChange?.(result.repos);
    } catch (err) {
      onError?.(errorMessage(err));
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
    try {
      await removeGitlabCredential(host);
      await loadConfiguredHosts();
    } catch (err) {
      onError?.(errorMessage(err));
    }
  };

  const handleUpdateCredential = (host: string) => {
    setKeysDialogOpen(false);
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

  const handleCopyRepo = (repo: SkillRepo) => {
    void handleCopyText(formatRepoSourceConfig(repo), repoShortPath(repo));
  };

  if (!open) return null;

  return (
    <>
      <div
        className="overlay drawer-overlay open"
        role="dialog"
        aria-modal="true"
        aria-label="来源管理"
        onClick={onClose}
      >
        <div className="drawer source-manage-drawer" onClick={(e) => e.stopPropagation()}>
          <div className="drawer-header-row">
            <div>
              <h2>来源管理</h2>
              <p className="drawer-subtitle">管理 Skill Hub、GitHub 与 GitLab 来源</p>
            </div>
            <div className="drawer-header-actions">
              <button type="button" className="btn-keys-link" onClick={() => setKeysDialogOpen(true)}>
                密钥管理
              </button>
              <button type="button" className="btn-primary" onClick={() => setAddModalOpen(true)}>
                添加来源
              </button>
            </div>
          </div>

          {loading ? (
            <p className="drawer-loading">加载中…</p>
          ) : (
            <ul className="repo-list source-list">
              {endpoints.length === 0 && repos.length === 0 ? (
                <li className="repo-empty">暂无来源，请点击「添加来源」。</li>
              ) : (
                <>
                  {endpoints.map((endpoint) => {
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
                          Hub
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
                            onClick={() => void handleRemoveHub(endpoint)}
                            aria-label={`删除 ${endpoint.name}`}
                          >
                            删除
                          </button>
                        </div>
                      </li>
                    );
                  })}
                  {repos.map((repo) => {
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
                            onClick={() => void handleRemoveRepo(repo)}
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
              resetAddForm();
            }
          }}
        >
          <div className="modal add-source-modal" onClick={(e) => e.stopPropagation()}>
            <h3>添加来源</h3>
            <div className="text-tabs add-source-tabs" role="tablist">
              {(['hub', 'github', 'gitlab'] as const).map((tab) => (
                <button
                  key={tab}
                  type="button"
                  role="tab"
                  className={`text-tab${addTab === tab ? ' active' : ''}`}
                  aria-selected={addTab === tab}
                  onClick={() => setAddTab(tab)}
                >
                  {tab === 'hub' ? 'Skill Hub' : tab === 'github' ? 'GitHub' : 'GitLab'}
                </button>
              ))}
            </div>

            {addTab === 'hub' ? (
              <div className="add-source-form">
                <label>
                  名称
                  <input
                    type="text"
                    value={hubName}
                    onChange={(e) => setHubName(e.target.value)}
                    placeholder="公司 Skill Hub"
                    disabled={adding}
                  />
                </label>
                <label>
                  Base URL
                  <input
                    type="text"
                    value={hubBaseUrl}
                    onChange={(e) => setHubBaseUrl(e.target.value)}
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
                    onChange={(e) => setRepoUrl(e.target.value)}
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

            <div className="modal-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => {
                  setAddModalOpen(false);
                  resetAddForm();
                }}
                disabled={adding}
              >
                取消
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => void (addTab === 'hub' ? handleAddHub() : handleAddRepo())}
                disabled={adding}
              >
                {adding ? '添加中…' : '添加'}
              </button>
            </div>
          </div>
        </div>
      )}

      <KeysManageDialog
        open={keysDialogOpen}
        hosts={configuredHosts}
        onClose={() => setKeysDialogOpen(false)}
        onUpdate={handleUpdateCredential}
        onRemove={handleRemoveCredential}
      />

      <GitLabPatDialog
        open={patDialog !== null}
        host={patDialog?.host ?? ''}
        description={
          patDialog?.mode === 'update'
            ? (patDialog?.host ?? '')
            : (patDialog?.projectPath ?? '')
        }
        mode={patDialog?.mode ?? 'add'}
        onClose={() => setPatDialog(null)}
        onSubmit={handlePatSubmit}
        submitLabel={patDialog?.mode === 'update' ? '验证并保存' : '验证并添加'}
      />
    </>
  );
}

export default SourceManageDrawer;
