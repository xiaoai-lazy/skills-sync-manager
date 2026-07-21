import type {
  DiscoverableSkill,
  IflytekSkillHubEndpoint,
  SkillHubEndpoint,
  SkillRecord,
  SkillRepo,
  SkillUpdateInfo,
  SkillView,
} from '../../model/types';

export const ALL_NODE_ID = 'all';
export const LOCAL_NODE_ID = 'local';
export const ALL_HUB_GROUP = 'all';
export const SKILLS_SYNC_NODE_ID = 'skillsSync';
export const IFLYTEK_ROOT_NODE_ID = 'iflytek';

const REMOTE_SOURCES = ['github', 'gitlab', 'skillhub', 'skillssh', 'iflytek'] as const;

export function resolveSkillRecord(
  skill: Pick<SkillView, 'storageKey' | 'dirName' | 'linkName'>,
  skillRecords?: Record<string, SkillRecord>,
): SkillRecord | undefined {
  if (!skillRecords) return undefined;
  if (skill.storageKey && skillRecords[skill.storageKey]) {
    return skillRecords[skill.storageKey];
  }
  // Legacy fallback (remove next major): flat-key records keyed by dirName.
  if (skill.dirName && skillRecords[skill.dirName]) {
    return skillRecords[skill.dirName];
  }
  return undefined;
}

function hubStoragePrefix(endpointId: string, group?: string): string {
  return group ? `hub/${endpointId}/${group}/` : `hub/${endpointId}/`;
}

function skillMatchesHubNode(
  skill: Pick<SkillView, 'storageKey'>,
  endpointId: string,
  group?: string,
): boolean {
  if (!skill.storageKey.startsWith('hub/')) {
    return false;
  }
  return skill.storageKey.startsWith(hubStoragePrefix(endpointId, group));
}

function updateMatchesSkill(
  update: SkillUpdateInfo,
  skill: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
): boolean {
  if (update.storageKey && skill.storageKey && update.storageKey === skill.storageKey) {
    return true;
  }

  const linkName = skill.linkName || skill.dirName;

  // Legacy cache: bare name (no '/') stored instead of full storageKey.
  if (update.storageKey && !update.storageKey.includes('/')) {
    return (
      skill.storageKey === update.storageKey ||
      skill.storageKey.endsWith(`/${update.storageKey}`) ||
      linkName === update.storageKey ||
      skill.dirName === update.storageKey
    );
  }

  // Legacy cache: empty storageKey — fall back to dir/link name only.
  if (!update.storageKey) {
    return update.dirName === skill.dirName || update.dirName === linkName;
  }

  return false;
}

export function findPendingUpdate(
  skill: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
  pendingUpdates: SkillUpdateInfo[],
): SkillUpdateInfo | undefined {
  return pendingUpdates.find((update) => updateMatchesSkill(update, skill));
}

export function skillHasPendingUpdate(
  skill: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
  pendingUpdates: SkillUpdateInfo[],
): boolean {
  return findPendingUpdate(skill, pendingUpdates) !== undefined;
}

/** Updates that correspond to at least one currently installed skill. */
export function matchedPendingUpdates(
  skills: Array<Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>>,
  pendingUpdates: SkillUpdateInfo[],
): SkillUpdateInfo[] {
  return pendingUpdates.filter((update) =>
    skills.some((skill) => updateMatchesSkill(update, skill)),
  );
}

export function pendingUpdateIdentifier(
  skill: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
  pendingUpdates: SkillUpdateInfo[],
): string {
  const match = findPendingUpdate(skill, pendingUpdates);
  if (match?.storageKey) return match.storageKey;
  if (skill.storageKey) return skill.storageKey;
  return skill.dirName;
}

export function hubRootNodeId(endpointId: string): string {
  return `hub:${endpointId}`;
}

export function hubGroupNodeId(endpointId: string, group: string): string {
  return `hub:${endpointId}:${group}`;
}

export function iflytekRootNodeId(endpointId: string): string {
  return `iflytek:${endpointId}`;
}

export function iflytekNamespaceNodeId(endpointId: string, namespace: string): string {
  return `iflytek:${endpointId}:${namespace}`;
}

export function resolveEffectiveFilterNodeId(
  selectedNodeId: string,
  hubGroup: string,
  endpoints: SkillHubEndpoint[],
  iflytekEndpoints: IflytekSkillHubEndpoint[] = [],
): string {
  if (hubGroup === ALL_HUB_GROUP) {
    return selectedNodeId;
  }
  const hub = parseHubNodeId(selectedNodeId);
  if (hub && !hub.group && isHubRootNode(selectedNodeId, endpoints)) {
    return hubGroupNodeId(hub.endpointId, hubGroup);
  }
  const iflytek = parseIflytekNodeId(selectedNodeId);
  if (
    iflytek &&
    !iflytek.namespace &&
    isIflytekRootNode(selectedNodeId, iflytekEndpoints)
  ) {
    return iflytekNamespaceNodeId(iflytek.endpointId, hubGroup);
  }
  return selectedNodeId;
}

export function repoNodeId(host: string, projectPath: string): string {
  return `repo:${host}/${projectPath}`;
}

export function parseHubNodeId(nodeId: string): { endpointId: string; group?: string } | null {
  if (!nodeId.startsWith('hub:')) return null;
  const rest = nodeId.slice(4);
  const colon = rest.indexOf(':');
  if (colon === -1) return { endpointId: rest };
  return { endpointId: rest.slice(0, colon), group: rest.slice(colon + 1) };
}

export function parseIflytekNodeId(
  nodeId: string,
): { endpointId: string; namespace?: string } | null {
  if (!nodeId.startsWith('iflytek:')) return null;
  const rest = nodeId.slice('iflytek:'.length);
  const colon = rest.indexOf(':');
  if (colon === -1) return { endpointId: rest };
  return { endpointId: rest.slice(0, colon), namespace: rest.slice(colon + 1) };
}

export function parseRepoNodeId(nodeId: string): { host: string; projectPath: string } | null {
  if (!nodeId.startsWith('repo:')) return null;
  const rest = nodeId.slice(5);
  const slash = rest.indexOf('/');
  if (slash === -1) return null;
  return { host: rest.slice(0, slash), projectPath: rest.slice(slash + 1) };
}

export function isHubRootNode(nodeId: string, endpoints: SkillHubEndpoint[]): boolean {
  const parsed = parseHubNodeId(nodeId);
  if (!parsed || parsed.group) return false;
  return endpoints.some((e) => e.id === parsed.endpointId);
}

export function isIflytekRootNode(
  nodeId: string,
  iflytekEndpoints: IflytekSkillHubEndpoint[],
): boolean {
  const parsed = parseIflytekNodeId(nodeId);
  if (!parsed || parsed.namespace) return false;
  return iflytekEndpoints.some((e) => e.id === parsed.endpointId);
}

export function isEnabledHubRootNode(nodeId: string, endpoints: SkillHubEndpoint[]): boolean {
  const parsed = parseHubNodeId(nodeId);
  if (!parsed || parsed.group) return false;
  const endpoint = endpoints.find((e) => e.id === parsed.endpointId);
  return endpoint?.enabled === true;
}

function recordMatchesRepo(record: SkillRecord, host: string, projectPath: string): boolean {
  return (
    (record.source === 'github' ||
      record.source === 'gitlab' ||
      record.source === 'skillssh') &&
    record.repoHost === host &&
    record.projectPath === projectPath
  );
}

export function hasInstalledSkillsForHub(
  endpointId: string,
  skillRecords: Record<string, SkillRecord>,
): boolean {
  return Object.values(skillRecords).some(
    (record) => record.source === 'skillhub' && record.hubEndpointId === endpointId,
  );
}

export function hasInstalledSkillsForRepo(
  host: string,
  projectPath: string,
  skillRecords: Record<string, SkillRecord>,
): boolean {
  return Object.values(skillRecords).some((record) =>
    recordMatchesRepo(record, host, projectPath),
  );
}

export function hubEndpointVisible(
  endpoint: SkillHubEndpoint,
  _skillRecords: Record<string, SkillRecord>,
): boolean {
  // Always list configured Hub endpoints (muted when disabled). Hiding disabled hubs
  // with zero installs made them look "missing" until 来源管理 refreshed the tree.
  void _skillRecords;
  return Boolean(endpoint.id);
}

export function repoVisible(repo: SkillRepo, skillRecords: Record<string, SkillRecord>): boolean {
  if (repo.enabled) return true;
  return hasInstalledSkillsForRepo(repo.host, repo.projectPath, skillRecords);
}

export function hasLocalInstalledSkills(
  skills: SkillView[],
  skillRecords: Record<string, SkillRecord>,
): boolean {
  return skills.some((skill) => isLocalInstalledSkill(skill.dirName, skillRecords, skill));
}

export function isLocalInstalledSkill(
  dirName: string,
  skillRecords?: Record<string, SkillRecord>,
  skill?: Pick<SkillView, 'dirName' | 'storageKey' | 'linkName'>,
): boolean {
  const record = skill
    ? resolveSkillRecord(skill, skillRecords)
    : skillRecords?.[dirName];
  if (!record) return true;
  return !(REMOTE_SOURCES as readonly string[]).includes(record.source);
}

export function hubGroupsForEndpoint(
  endpointId: string,
  discoverSkills: DiscoverableSkill[],
  skillRecords: Record<string, SkillRecord>,
): string[] {
  const groups = new Set<string>();
  discoverSkills
    .filter(
      (skill) =>
        skill.source === 'skillhub' &&
        skill.hubEndpointId === endpointId &&
        skill.hubSkillGroup,
    )
    .forEach((skill) => groups.add(skill.hubSkillGroup));

  Object.values(skillRecords)
    .filter(
      (record) =>
        record.source === 'skillhub' &&
        record.hubEndpointId === endpointId &&
        record.hubSkillGroup,
    )
    .forEach((record) => groups.add(record.hubSkillGroup));

  return [...groups].sort();
}

export function iflytekNamespacesForEndpoint(
  endpointId: string,
  discoverSkills: DiscoverableSkill[],
  skillRecords: Record<string, SkillRecord>,
): string[] {
  const namespaces = new Set<string>();
  discoverSkills
    .filter(
      (skill) =>
        skill.source === 'iflytek' &&
        skill.hubEndpointId === endpointId &&
        skill.hubSkillGroup,
    )
    .forEach((skill) => namespaces.add(skill.hubSkillGroup));

  Object.values(skillRecords)
    .filter(
      (record) =>
        record.source === 'iflytek' &&
        record.hubEndpointId === endpointId &&
        record.hubSkillGroup,
    )
    .forEach((record) => namespaces.add(record.hubSkillGroup));

  return [...namespaces].sort();
}

export function countInstalledForNode(
  nodeId: string,
  skills: SkillView[],
  skillRecords: Record<string, SkillRecord>,
): number {
  return skills.filter((skill) =>
    matchesInstalledNode(
      nodeId,
      skill.dirName,
      resolveSkillRecord(skill, skillRecords),
      skill,
    ),
  ).length;
}

export function countDiscoverForNode(
  nodeId: string,
  discoverSkills: DiscoverableSkill[],
): number {
  return discoverSkills.filter((skill) => matchesDiscoverNode(nodeId, skill)).length;
}

export function matchesInstalledNode(
  nodeId: string,
  dirName: string,
  record?: SkillRecord,
  skill?: Pick<SkillView, 'storageKey' | 'linkName'>,
): boolean {
  if (nodeId === ALL_NODE_ID) return true;

  if (nodeId === LOCAL_NODE_ID) {
    if (!record) return true;
    return !(REMOTE_SOURCES as readonly string[]).includes(record.source);
  }

  if (nodeId === SKILLS_SYNC_NODE_ID) {
    return record?.source === 'skillhub';
  }

  if (nodeId === IFLYTEK_ROOT_NODE_ID) {
    return record?.source === 'iflytek';
  }

  const hub = parseHubNodeId(nodeId);
  if (hub) {
    if (record) {
      if (record.source !== 'skillhub' || record.hubEndpointId !== hub.endpointId) {
        return false;
      }
      if (hub.group) return record.hubSkillGroup === hub.group;
      return true;
    }
    if (skill?.storageKey) {
      return skillMatchesHubNode(skill, hub.endpointId, hub.group);
    }
    return false;
  }

  const iflytek = parseIflytekNodeId(nodeId);
  if (iflytek) {
    if (!record || record.source !== 'iflytek' || record.hubEndpointId !== iflytek.endpointId) {
      return false;
    }
    if (iflytek.namespace) return record.hubSkillGroup === iflytek.namespace;
    return true;
  }

  const repo = parseRepoNodeId(nodeId);
  if (repo) {
    if (skill?.storageKey?.startsWith('repo/')) {
      const repoSlug = skill.storageKey.slice('repo/'.length).split('/')[0];
      const expectedSlug = `${repo.host}--${repo.projectPath.replace(/\//g, '-')}`;
      if (repoSlug !== expectedSlug) {
        return false;
      }
    }
    if (!record) return false;
    return recordMatchesRepo(record, repo.host, repo.projectPath);
  }

  return true;
}

export function dedupeInstalledSkills(skills: SkillView[]): SkillView[] {
  const seen = new Set<string>();
  const deduped: SkillView[] = [];
  for (const skill of skills) {
    const key = skill.storageKey || skill.dirName;
    if (!key || seen.has(key)) continue;
    seen.add(key);
    deduped.push(skill);
  }
  return deduped;
}

export function matchesDiscoverNode(nodeId: string, skill: DiscoverableSkill): boolean {
  if (nodeId === ALL_NODE_ID) return true;
  if (nodeId === LOCAL_NODE_ID) return false;

  if (nodeId === SKILLS_SYNC_NODE_ID) {
    return skill.source === 'skillhub';
  }

  if (nodeId === IFLYTEK_ROOT_NODE_ID) {
    return skill.source === 'iflytek';
  }

  const hub = parseHubNodeId(nodeId);
  if (hub) {
    if (skill.source !== 'skillhub' || skill.hubEndpointId !== hub.endpointId) return false;
    if (hub.group) return skill.hubSkillGroup === hub.group;
    return true;
  }

  const iflytek = parseIflytekNodeId(nodeId);
  if (iflytek) {
    if (skill.source !== 'iflytek' || skill.hubEndpointId !== iflytek.endpointId) return false;
    if (iflytek.namespace) return skill.hubSkillGroup === iflytek.namespace;
    return true;
  }

  const repo = parseRepoNodeId(nodeId);
  if (repo) {
    return (
      (skill.source === 'github' || skill.source === 'gitlab') &&
      skill.repoHost === repo.host &&
      skill.projectPath === repo.projectPath
    );
  }

  return true;
}

export function nodeTitle(
  nodeId: string,
  endpoints: SkillHubEndpoint[],
  repos: SkillRepo[],
  iflytekEndpoints: IflytekSkillHubEndpoint[] = [],
): { title: string; sub: string } {
  if (nodeId === ALL_NODE_ID) {
    return { title: '全部 Skill', sub: '所有来源' };
  }
  if (nodeId === LOCAL_NODE_ID) {
    return { title: '本地导入', sub: '手动导入的 Skill' };
  }
  if (nodeId === SKILLS_SYNC_NODE_ID) {
    return { title: 'Skills Sync Hub', sub: '企业内部 Skills Sync' };
  }
  if (nodeId === IFLYTEK_ROOT_NODE_ID) {
    return { title: 'iFlytek Skill Hub', sub: '讯飞 Skill Hub' };
  }

  const hub = parseHubNodeId(nodeId);
  if (hub) {
    const endpoint = endpoints.find((e) => e.id === hub.endpointId);
    const name = endpoint?.name ?? hub.endpointId;
    if (hub.group) return { title: hub.group, sub: `${name} · 分组` };
    return {
      title: name,
      sub: `Skills Sync Hub${endpoint?.baseUrl ? ` · ${endpoint.baseUrl}` : ''}`,
    };
  }

  const iflytek = parseIflytekNodeId(nodeId);
  if (iflytek) {
    const endpoint = iflytekEndpoints.find((e) => e.id === iflytek.endpointId);
    const name = endpoint?.name ?? iflytek.endpointId;
    if (iflytek.namespace) return { title: iflytek.namespace, sub: `${name} · 命名空间` };
    return {
      title: name,
      sub: `iFlytek Skill Hub${endpoint?.baseUrl ? ` · ${endpoint.baseUrl}` : ''}`,
    };
  }

  const repo = parseRepoNodeId(nodeId);
  if (repo) {
    const match = repos.find(
      (r) => r.host === repo.host && r.projectPath === repo.projectPath,
    );
    const label =
      match?.provider === 'gitlab'
        ? `${repo.host}/${repo.projectPath}`
        : repo.projectPath;
    return {
      title: match?.name ?? label,
      sub: match?.provider === 'gitlab' ? 'GitLab' : 'GitHub',
    };
  }

  return { title: 'Skill', sub: '' };
}
