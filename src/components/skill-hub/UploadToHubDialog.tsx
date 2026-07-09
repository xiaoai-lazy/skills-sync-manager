import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  createHubGroup,
  listHubGroups,
  uploadSkillToHub,
  checkSkillUpdates,
} from '../../api/skillHub';
import { errorMessage } from '../../utils/errorMessage';
import type {
  SkillHubEndpoint,
  SkillHubLocalState,
  SkillRecord,
  SkillView,
} from '../../model/types';
import { repoNodeId, resolveSkillRecord } from './sourceTreeUtils';

export interface UploadToHubDialogProps {
  open: boolean;
  hubEndpointId: string;
  hubEndpointName: string;
  hubState: SkillHubLocalState;
  skillRecords: Record<string, SkillRecord>;
  enabledHubEndpoints: SkillHubEndpoint[];
  onClose: () => void;
  onDiscoverSkillsChange: (skills: import('../../model/types').DiscoverableSkill[]) => void;
  onPendingUpdatesChange?: (updates: import('../../model/types').SkillUpdateInfo[]) => void;
  onRefreshHubState?: () => Promise<void>;
  onToast?: (message: string) => void;
  onError?: (error: unknown) => void;
}

type UploadBucketKind = 'hub' | 'repo' | 'local';

interface UploadBucket {
  kind: UploadBucketKind;
  sourceId: string;
  label: string;
}

interface InstalledPick {
  dirName: string;
  name: string;
  storageKey: string;
  group?: string;
  skillId: string;
}

function resolveUploadSkillId(skill: SkillView, record?: SkillRecord): string {
  if (record?.linkName) return record.linkName;
  const displayName = skill.name?.trim();
  if (displayName) return displayName;
  return skill.dirName;
}

function skillOptionLabel(pick: InstalledPick): string {
  if (pick.name && pick.name !== pick.skillId) {
    return `${pick.name} (${pick.skillId})`;
  }
  return pick.skillId;
}

function buildUploadBuckets(
  skills: SkillView[],
  skillRecords: Record<string, SkillRecord>,
  endpoints: SkillHubEndpoint[],
  repos: { host: string; projectPath: string; label: string }[],
): UploadBucket[] {
  const buckets: UploadBucket[] = [];
  const seen = new Set<string>();

  for (const skill of skills) {
    const record = resolveSkillRecord(skill, skillRecords);
    if (!record) {
      if (!seen.has('local')) {
        seen.add('local');
        buckets.push({ kind: 'local', sourceId: 'local', label: '本地导入' });
      }
      continue;
    }

    if (record.source === 'skillhub' && record.hubEndpointId) {
      const endpoint = endpoints.find((e) => e.id === record.hubEndpointId);
      const key = `hub:${record.hubEndpointId}`;
      if (!seen.has(key)) {
        seen.add(key);
        buckets.push({
          kind: 'hub',
          sourceId: record.hubEndpointId,
          label: endpoint?.name ?? record.hubEndpointId,
        });
      }
      continue;
    }

    if (
      (record.source === 'github' || record.source === 'gitlab' || record.source === 'skillssh') &&
      record.repoHost &&
      record.projectPath
    ) {
      const key = repoNodeId(record.repoHost, record.projectPath);
      if (!seen.has(key)) {
        seen.add(key);
        const repo = repos.find(
          (r) => r.host === record.repoHost && r.projectPath === record.projectPath,
        );
        buckets.push({
          kind: 'repo',
          sourceId: key,
          label: repo?.label ?? record.projectPath,
        });
      }
    }
  }

  if (!seen.has('local') && skills.some((s) => !resolveSkillRecord(s, skillRecords))) {
    buckets.push({ kind: 'local', sourceId: 'local', label: '本地导入' });
  }

  return buckets;
}

function skillsForBucket(
  bucket: UploadBucket,
  skills: SkillView[],
  skillRecords: Record<string, SkillRecord>,
  group?: string,
): InstalledPick[] {
  return skills
    .filter((skill) => {
      const record = resolveSkillRecord(skill, skillRecords);
      if (bucket.kind === 'local') {
        return !record || !['github', 'gitlab', 'skillhub', 'skillssh'].includes(record.source);
      }
      if (bucket.kind === 'hub') {
        if (!record || record.source !== 'skillhub' || record.hubEndpointId !== bucket.sourceId) {
          return false;
        }
        if (group) return record.hubSkillGroup === group;
        return true;
      }
      const repo = bucket.sourceId.startsWith('repo:') ? bucket.sourceId.slice(5) : '';
      const slash = repo.indexOf('/');
      const host = repo.slice(0, slash);
      const projectPath = repo.slice(slash + 1);
      if (!record) return false;
      return (
        (record.source === 'github' ||
          record.source === 'gitlab' ||
          record.source === 'skillssh') &&
        record.repoHost === host &&
        record.projectPath === projectPath
      );
    })
    .map((skill) => {
      const record = resolveSkillRecord(skill, skillRecords);
      return {
        dirName: skill.dirName,
        name: skill.name ?? skill.dirName,
        storageKey: skill.storageKey || record?.storageKey || '',
        group: record?.hubSkillGroup,
        skillId: resolveUploadSkillId(skill, record),
      };
    })
    .filter((item) => Boolean(item.storageKey));
}

function UploadToHubDialog(props: UploadToHubDialogProps) {
  const {
    open,
    hubEndpointId,
    hubState,
    skillRecords,
    enabledHubEndpoints,
    onClose,
    onDiscoverSkillsChange,
    onPendingUpdatesChange,
    onRefreshHubState,
    onToast,
    onError,
  } = props;

  const [targetHubId, setTargetHubId] = useState(hubEndpointId);
  const [groups, setGroups] = useState<string[]>([]);
  const [targetGroup, setTargetGroup] = useState('');
  const [loadingGroups, setLoadingGroups] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [creatingGroup, setCreatingGroup] = useState(false);
  const [newGroupDialogOpen, setNewGroupDialogOpen] = useState(false);
  const [srcBucketId, setSrcBucketId] = useState('');
  const [srcGroup, setSrcGroup] = useState('');
  const [srcSkillKey, setSrcSkillKey] = useState('');

  const repoLabels = useMemo(
    () =>
      Object.values(skillRecords)
        .filter((r) => r.repoHost && r.projectPath)
        .map((r) => ({
          host: r.repoHost,
          projectPath: r.projectPath,
          label: r.projectPath,
        })),
    [skillRecords],
  );

  const buckets = useMemo(
    () => buildUploadBuckets(hubState.skills, skillRecords, enabledHubEndpoints, repoLabels),
    [hubState.skills, skillRecords, enabledHubEndpoints, repoLabels],
  );

  const selectedBucket = buckets.find((b) => b.sourceId === srcBucketId) ?? buckets[0];

  const srcGroups = useMemo(() => {
    if (!selectedBucket || selectedBucket.kind !== 'hub') return [];
    const groupsSet = new Set<string>();
    skillsForBucket(selectedBucket, hubState.skills, skillRecords).forEach((skill) => {
      if (skill.group) groupsSet.add(skill.group);
    });
    return [...groupsSet].sort();
  }, [selectedBucket, hubState.skills, skillRecords]);

  const srcSkills = useMemo(() => {
    if (!selectedBucket) return [];
    const group = selectedBucket.kind === 'hub' ? srcGroup : undefined;
    return skillsForBucket(selectedBucket, hubState.skills, skillRecords, group || undefined);
  }, [selectedBucket, hubState.skills, skillRecords, srcGroup]);

  const selectedSkill = srcSkills.find((s) => s.storageKey === srcSkillKey) ?? srcSkills[0];

  const loadTargetGroups = useCallback(
    async (endpointId: string) => {
      setLoadingGroups(true);
      try {
        const list = await listHubGroups(endpointId);
        setGroups(list);
        setTargetGroup((prev) => (list.includes(prev) ? prev : (list[0] ?? '')));
      } catch (err) {
        onError?.(errorMessage(err));
        setGroups([]);
        setTargetGroup('');
      } finally {
        setLoadingGroups(false);
      }
    },
    [onError],
  );

  useEffect(() => {
    if (!open) return;
    setTargetHubId(hubEndpointId);
    setNewGroupDialogOpen(false);
    setNewGroupName('');
  }, [open, hubEndpointId]);

  useEffect(() => {
    if (!open || !targetHubId) return;
    void loadTargetGroups(targetHubId);
  }, [open, targetHubId, loadTargetGroups]);

  useEffect(() => {
    if (!open) return;
    if (buckets.length > 0 && !buckets.some((b) => b.sourceId === srcBucketId)) {
      setSrcBucketId(buckets[0].sourceId);
    }
  }, [open, buckets, srcBucketId]);

  useEffect(() => {
    if (!selectedBucket || selectedBucket.kind !== 'hub') {
      setSrcGroup('');
      return;
    }
    if (srcGroups.length > 0 && !srcGroups.includes(srcGroup)) {
      setSrcGroup(srcGroups[0]);
    }
  }, [selectedBucket, srcGroups, srcGroup]);

  useEffect(() => {
    if (srcSkills.length > 0 && !srcSkills.some((s) => s.storageKey === srcSkillKey)) {
      setSrcSkillKey(srcSkills[0].storageKey);
    }
  }, [srcSkills, srcSkillKey]);

  const handleCreateGroup = async () => {
    const name = newGroupName.trim();
    if (!name || !targetHubId || creatingGroup) return;
    setCreatingGroup(true);
    try {
      const list = await createHubGroup(targetHubId, name);
      setGroups(list);
      setTargetGroup(name);
      setNewGroupName('');
      setNewGroupDialogOpen(false);
      onToast?.('分组已创建');
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setCreatingGroup(false);
    }
  };

  const handleUpload = async () => {
    if (!selectedSkill?.storageKey || !targetHubId || !targetGroup || uploading) return;
    setUploading(true);
    try {
      const result = await uploadSkillToHub(targetHubId, targetGroup, selectedSkill.storageKey);
      onDiscoverSkillsChange(result.discoverSkills);
      const updates = await checkSkillUpdates();
      onPendingUpdatesChange?.(updates);
      await onRefreshHubState?.();
      onToast?.('上传成功，列表已刷新');
      onClose();
    } catch (err) {
      onError?.(errorMessage(err));
    } finally {
      setUploading(false);
    }
  };

  if (!open) return null;

  return (
    <>
      <div
        className="overlay open upload-hub-overlay"
        role="dialog"
        aria-modal="true"
        aria-label="上传到 Hub"
        onClick={onClose}
      >
      <div className="modal upload-hub-modal" onClick={(e) => e.stopPropagation()}>
        <h3 className="upload-hub-title">上传到 Hub</h3>

        <div className="upload-hub-form">
          {buckets.length > 0 ? (
            <div className="upload-row">
              <label className="upload-row-label" htmlFor="upload-src-bucket">
                来源
              </label>
              <select
                id="upload-src-bucket"
                className="upload-control"
                value={srcBucketId}
                onChange={(e) => setSrcBucketId(e.target.value)}
                disabled={uploading || buckets.length <= 1}
              >
                {buckets.map((bucket) => (
                  <option key={bucket.sourceId} value={bucket.sourceId}>
                    {bucket.label}
                  </option>
                ))}
              </select>
            </div>
          ) : null}

          {selectedBucket?.kind === 'hub' ? (
            <div className="upload-row">
              <label className="upload-row-label" htmlFor="upload-src-group">
                来源分组
              </label>
              <select
                id="upload-src-group"
                className="upload-control"
                value={srcGroup}
                onChange={(e) => setSrcGroup(e.target.value)}
                disabled={uploading || srcGroups.length === 0}
              >
                {srcGroups.length === 0 ? (
                  <option value="">（无分组）</option>
                ) : (
                  srcGroups.map((group) => (
                    <option key={group} value={group}>
                      {group}
                    </option>
                  ))
                )}
              </select>
            </div>
          ) : null}

          <div className="upload-row">
            <label className="upload-row-label" htmlFor="upload-skill">
              Skill
            </label>
            <select
              id="upload-skill"
              className="upload-control"
              value={srcSkillKey}
              onChange={(e) => setSrcSkillKey(e.target.value)}
              disabled={uploading || srcSkills.length === 0}
            >
              {srcSkills.length === 0 ? (
                <option value="">（无）</option>
              ) : (
                srcSkills.map((skill) => (
                  <option key={skill.storageKey} value={skill.storageKey}>
                    {skillOptionLabel(skill)}
                  </option>
                ))
              )}
            </select>
          </div>

          {enabledHubEndpoints.length > 1 ? (
            <div className="upload-row">
              <label className="upload-row-label" htmlFor="upload-target-hub">
                目标 Hub
              </label>
              <select
                id="upload-target-hub"
                className="upload-control"
                value={targetHubId}
                onChange={(e) => setTargetHubId(e.target.value)}
                disabled={uploading}
              >
                {enabledHubEndpoints.map((endpoint) => (
                  <option key={endpoint.id} value={endpoint.id}>
                    {endpoint.name}
                  </option>
                ))}
              </select>
            </div>
          ) : null}

          <div className="upload-row">
            <label className="upload-row-label" htmlFor="upload-target-group">
              目标分组
            </label>
            <div className="upload-row-controls">
              <select
                id="upload-target-group"
                className="upload-control"
                value={targetGroup}
                onChange={(e) => setTargetGroup(e.target.value)}
                disabled={uploading || loadingGroups || groups.length === 0}
              >
                {groups.length === 0 ? (
                  <option value="">{loadingGroups ? '加载中…' : '（无分组）'}</option>
                ) : (
                  groups.map((group) => (
                    <option key={group} value={group}>
                      {group}
                    </option>
                  ))
                )}
              </select>
              <button
                type="button"
                className="upload-link-btn"
                onClick={() => {
                  setNewGroupName('');
                  setNewGroupDialogOpen(true);
                }}
                disabled={uploading || creatingGroup}
              >
                新建
              </button>
            </div>
          </div>
        </div>

        <div className="modal-actions upload-hub-actions">
          <button type="button" className="secondary-button" onClick={onClose} disabled={uploading}>
            取消
          </button>
          <button
            type="button"
            className="btn-primary btn-hub"
            onClick={() => void handleUpload()}
            disabled={
              uploading || !selectedSkill?.storageKey || !targetGroup || srcSkills.length === 0
            }
          >
            {uploading ? '上传中…' : '上传'}
          </button>
        </div>
      </div>
      </div>

      {newGroupDialogOpen ? (
        <div
          className="overlay open upload-nested-overlay"
          role="dialog"
          aria-modal="true"
          aria-label="新建分组"
          onClick={() => {
            if (!creatingGroup) setNewGroupDialogOpen(false);
          }}
        >
          <div className="modal upload-nested-modal" onClick={(e) => e.stopPropagation()}>
            <h3 className="upload-hub-title">新建分组</h3>
            <div className="dialog-form-field">
              <label htmlFor="upload-new-group-name">分组名称</label>
              <input
                id="upload-new-group-name"
                type="text"
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                placeholder="例如 review、common"
                disabled={creatingGroup}
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && newGroupName.trim()) void handleCreateGroup();
                }}
              />
            </div>
            <div className="modal-actions upload-hub-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setNewGroupDialogOpen(false)}
                disabled={creatingGroup}
              >
                取消
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => void handleCreateGroup()}
                disabled={creatingGroup || !newGroupName.trim()}
              >
                {creatingGroup ? '创建中…' : '创建'}
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}

export default UploadToHubDialog;
