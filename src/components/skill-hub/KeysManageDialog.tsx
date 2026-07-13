import { useEffect, useMemo } from 'react';
import type { SkillRepo } from '../../model/types';

export interface KeysManageDialogProps {
  open: boolean;
  repos: SkillRepo[];
  configuredHosts: string[];
  nestedDialogOpen: boolean;
  onClose: () => void;
  onAuthenticate: (host: string) => void;
  onUpdate: (host: string) => void;
  onRemove: (host: string) => Promise<void>;
}

interface GitLabHostGroup {
  host: string;
  repos: SkillRepo[];
  authenticated: boolean;
}

function normalizeHost(host: string): string {
  return host.trim().toLowerCase();
}

function groupGitLabRepos(repos: SkillRepo[], configuredHosts: string[]): GitLabHostGroup[] {
  const authenticatedHosts = new Set(configuredHosts.map(normalizeHost));
  const grouped = new Map<string, SkillRepo[]>();

  for (const repo of repos) {
    if (repo.provider !== 'gitlab') continue;
    const host = normalizeHost(repo.host);
    if (!host) continue;
    grouped.set(host, [...(grouped.get(host) ?? []), repo]);
  }

  return [...grouped.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([host, hostRepos]) => ({
      host,
      repos: [...hostRepos].sort((left, right) =>
        left.projectPath.localeCompare(right.projectPath),
      ),
      authenticated: authenticatedHosts.has(host),
    }));
}

function KeysManageDialog(props: KeysManageDialogProps) {
  const {
    open,
    repos,
    configuredHosts,
    nestedDialogOpen,
    onClose,
    onAuthenticate,
    onUpdate,
  } = props;
  const groups = useMemo(
    () => groupGitLabRepos(repos, configuredHosts),
    [repos, configuredHosts],
  );

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !nestedDialogOpen) onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose, nestedDialogOpen]);

  if (!open) return null;

  return (
    <div
      className="modal-overlay open"
      role="dialog"
      aria-modal="true"
      aria-labelledby="keysModalTitle"
      onClick={onClose}
    >
      <div className="modal modal-wide" onClick={(event) => event.stopPropagation()}>
        <h3 id="keysModalTitle">密钥管理</h3>
        <p className="keys-hint">
          GitLab 访问密钥按站点域名保存，同一站点下的来源仓库共用密钥。
        </p>
        {groups.length === 0 ? (
          <p className="keys-empty">暂无已添加的 GitLab 来源仓库</p>
        ) : (
          <div className="credential-host-list">
            {groups.map((group) => (
              <section
                key={group.host}
                className="credential-host"
                data-testid={`gitlab-host-${group.host}`}
              >
                <div className="credential-host-header">
                  <div>
                    <strong>{group.host}</strong>
                    <div
                      className={`credential-status${group.authenticated ? ' authenticated' : ''}`}
                    >
                      {group.authenticated ? '已认证' : '未配置认证'} ·{' '}
                      <span>{group.repos.length} 个仓库</span>
                    </div>
                  </div>
                  <div className="key-actions">
                    {group.authenticated ? (
                      <button type="button" onClick={() => onUpdate(group.host)}>
                        更新
                      </button>
                    ) : (
                      <button
                        type="button"
                        className="btn-primary"
                        onClick={() => onAuthenticate(group.host)}
                      >
                        去认证
                      </button>
                    )}
                  </div>
                </div>
                <ul className="credential-repo-list">
                  {group.repos.map((repo) => (
                    <li key={repo.projectPath}>{repo.projectPath}</li>
                  ))}
                </ul>
              </section>
            ))}
          </div>
        )}
        <div className="modal-actions">
          <button type="button" className="cancel" onClick={onClose}>
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}

export default KeysManageDialog;
