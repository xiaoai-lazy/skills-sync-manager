import React, { useEffect, useState } from 'react';
import { addSkillRepo, getSkillRepos, removeSkillRepo } from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import type { DiscoverableSkill, SkillRepo } from '../../model/types';

export interface RepoDrawerProps {
  open: boolean;
  onClose: () => void;
  onError?: (error: unknown) => void;
  onDiscoverSkillsChange?: (skills: DiscoverableSkill[]) => void;
}

function RepoDrawer(props: RepoDrawerProps) {
  const { open, onClose, onError, onDiscoverSkillsChange } = props;
  const [repos, setRepos] = useState<SkillRepo[]>([]);
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    if (!open) return;

    setLoading(true);
    getSkillRepos()
      .then(setRepos)
      .catch((err) => onError?.(errorMessage(err)))
      .finally(() => setLoading(false));
  }, [open, onError]);

  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  const handleAdd = async () => {
    const value = url.trim();
    if (!value) return;

    setAdding(true);
    try {
      const result = await addSkillRepo(value);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
      setUrl('');
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setAdding(false);
    }
  };

  const handleRemove = async (owner: string, name: string) => {
    try {
      const result = await removeSkillRepo(owner, name);
      setRepos(result.repos);
      onDiscoverSkillsChange?.(result.discoverSkills);
    } catch (err) {
      onError?.(errorMessage(err));
    }
  };

  if (!open) return null;

  return (
    <div
      className="overlay drawer-overlay open"
      role="dialog"
      aria-modal="true"
      aria-label="仓库管理"
      onClick={onClose}
    >
      <div className="drawer" onClick={(e) => e.stopPropagation()}>
        <h2>仓库管理</h2>
        <div className="repo-add-row">
          <input
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void handleAdd();
            }}
            placeholder="GitHub 仓库 URL"
            aria-label="GitHub 仓库 URL"
            disabled={adding}
          />
          <button
            type="button"
            className="btn-primary"
            onClick={() => void handleAdd()}
            disabled={adding}
          >
            添加
          </button>
        </div>
        {loading ? (
          <p className="drawer-loading">加载中…</p>
        ) : (
          <ul className="repo-list">
            {repos.length === 0 ? (
              <li className="repo-empty">暂无仓库，请添加 GitHub 仓库。</li>
            ) : (
              repos.map((repo) => (
                <li key={`${repo.owner}/${repo.name}`} className="repo-item">
                  <div className="repo-item-info">
                    <strong className="repo-item-name" title={`${repo.owner}/${repo.name}`}>
                      {repo.owner}/{repo.name}
                    </strong>
                    <div className="repo-item-meta">{repo.branch}</div>
                  </div>
                  <button
                    type="button"
                    className="btn-sm btn-danger"
                    onClick={() => void handleRemove(repo.owner, repo.name)}
                  >
                    删除
                  </button>
                </li>
              ))
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
  );
}

export default RepoDrawer;
