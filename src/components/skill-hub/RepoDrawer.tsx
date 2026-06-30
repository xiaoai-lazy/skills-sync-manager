import React, { useCallback, useEffect, useState } from 'react';
import {
  addSkillRepo,
  getSkillRepos,
  listGitlabCredentials,
  previewAddSkillRepo,
  removeGitlabCredential,
  removeSkillRepo,
  setSkillRepoEnabled,
  updateGitlabCredential,
  validateGitlabPat,
} from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import type { DiscoverableSkill, SkillRepo } from '../../model/types';
import GitLabPatDialog from './GitLabPatDialog';
import KeysManageDialog from './KeysManageDialog';

export interface RepoDrawerProps {
  open: boolean;
  onClose: () => void;
  onError?: (error: unknown) => void;
  onDiscoverSkillsChange?: (skills: DiscoverableSkill[]) => void;
}

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

function RepoDrawer(props: RepoDrawerProps) {
  const { open, onClose, onError, onDiscoverSkillsChange } = props;
  const [repos, setRepos] = useState<SkillRepo[]>([]);
  const [configuredHosts, setConfiguredHosts] = useState<string[]>([]);
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);
  const [keysDialogOpen, setKeysDialogOpen] = useState(false);
  const [patDialog, setPatDialog] = useState<PatDialogState | null>(null);
  const [togglingRepoKey, setTogglingRepoKey] = useState<string | null>(null);

  const loadConfiguredHosts = useCallback(async () => {
    try {
      const hosts = await listGitlabCredentials();
      setConfiguredHosts(hosts);
    } catch (err) {
      onError?.(errorMessage(err));
    }
  }, [onError]);

  const loadRepos = useCallback(async () => {
    setLoading(true);
    try {
      const list = await getSkillRepos();
      setRepos(list);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [onError]);

  useEffect(() => {
    if (!open) return;
    void loadRepos();
    void loadConfiguredHosts();
  }, [open, loadRepos, loadConfiguredHosts]);

  useEffect(() => {
    if (!keysDialogOpen) return;
    void loadConfiguredHosts();
  }, [keysDialogOpen, loadConfiguredHosts]);

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !patDialog && !keysDialogOpen) onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose, patDialog, keysDialogOpen]);

  const finishAdd = async (addUrl: string, pat?: string) => {
    setAdding(true);
    try {
      const result = await addSkillRepo(addUrl, undefined, pat);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
      setUrl('');
      await loadConfiguredHosts();
    } catch (err) {
      onError?.(errorMessage(err));
      throw err;
    } finally {
      setAdding(false);
    }
  };

  const handleAdd = async () => {
    const value = url.trim();
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
      await finishAdd(value);
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

    // 先持久化密钥，再添加仓库；避免添加流程卡在 discover 时密钥列表不更新
    await updateGitlabCredential(host, pat);
    await loadConfiguredHosts();
    void finishAdd(addUrl);
  };

  const handleRemove = async (repo: SkillRepo) => {
    try {
      const result = await removeSkillRepo(repo.host, repo.projectPath);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
    }
  };

  const handleToggleEnabled = async (repo: SkillRepo, enabled: boolean) => {
    const key = repoItemKey(repo);
    if (togglingRepoKey) return;

    setTogglingRepoKey(key);
    try {
      const result = await setSkillRepoEnabled(repo.host, repo.projectPath, enabled);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setTogglingRepoKey(null);
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

  if (!open) return null;

  return (
    <>
      <div
        className="overlay drawer-overlay open"
        role="dialog"
        aria-modal="true"
        aria-label="Skill 来源仓库"
        onClick={onClose}
      >
        <div className="drawer" onClick={(e) => e.stopPropagation()}>
          <div className="drawer-header-row">
            <div>
              <h2>Skill 来源仓库</h2>
              <p className="drawer-subtitle">从 GitHub 或 GitLab 发现并安装 Skill</p>
            </div>
            <button
              type="button"
              className="btn-keys-link"
              onClick={() => setKeysDialogOpen(true)}
            >
              密钥管理
            </button>
          </div>

          <div className="repo-add-row">
            <input
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void handleAdd();
              }}
              placeholder="粘贴仓库链接，如 github.com/... 或 gitlab.example.com/..."
              aria-label="仓库链接"
              disabled={adding}
            />
            <button
              type="button"
              className="btn-primary"
              onClick={() => void handleAdd()}
              disabled={adding}
            >
              {adding ? '添加中…' : '添加来源'}
            </button>
          </div>

          {loading ? (
            <p className="drawer-loading">加载中…</p>
          ) : (
            <ul className="repo-list">
              {repos.length === 0 ? (
                <li className="repo-empty">暂无来源仓库，请添加 GitHub 或 GitLab 仓库。</li>
              ) : (
                repos.map((repo) => {
                  const path = repoShortPath(repo);
                  const authed =
                    repo.provider === 'gitlab' && configuredHosts.includes(repo.host);
                  const itemKey = repoItemKey(repo);
                  const toggling = togglingRepoKey === itemKey;
                  return (
                    <li
                      key={itemKey}
                      className={`repo-item${repo.enabled ? '' : ' repo-item-disabled'}`}
                    >
                      <span
                        className={`repo-provider-tag repo-provider-${repo.provider}`}
                        aria-hidden
                      >
                        {repoProviderName(repo)}
                      </span>
                      <div className="repo-item-info">
                        <div className="repo-item-name" title={path}>
                          {path}
                        </div>
                        <div className="repo-item-meta-row">
                          <span className="repo-branch-chip">{repo.branch}</span>
                          {authed && <span className="badge-auth">已认证</span>}
                          {!repo.enabled && <span className="badge-disabled">已停用</span>}
                        </div>
                      </div>
                      <div className="repo-item-actions">
                        <label
                          className="repo-switch"
                          title={repo.enabled ? '停用此来源' : '启用此来源'}
                        >
                          <input
                            type="checkbox"
                            checked={repo.enabled}
                            disabled={toggling}
                            onChange={(e) => void handleToggleEnabled(repo, e.target.checked)}
                            aria-label={`${repo.enabled ? '停用' : '启用'} ${path}`}
                          />
                          <span className="repo-switch-slider" aria-hidden />
                        </label>
                        <button
                          type="button"
                          className="btn-repo-remove"
                          onClick={() => void handleRemove(repo)}
                          aria-label={`删除 ${path}`}
                        >
                          删除
                        </button>
                      </div>
                    </li>
                  );
                })
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

export default RepoDrawer;
