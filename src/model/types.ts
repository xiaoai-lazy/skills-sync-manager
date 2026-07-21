export type LinkStrategy = 'auto'; // currently only auto; reserved for future strategies

export type LinkType = 'junction' | 'symlink';

export type SkillInstallState =
  | 'notInstalled'
  | 'installed'
  | 'conflict'
  | 'missing'
  | 'mismatch'
  | 'sourceMissing'
  | 'invalidSkill';

export interface AppErrorDto {
  code: string;
  message: string;
}

export interface Settings {
  mainSkillsDir: string | null;
  linkStrategy: LinkStrategy;
  startupRefresh: StartupRefreshSettings;
}

export interface StartupRefreshSettings {
  github: boolean;
  gitlab: boolean;
  skillHub: boolean;
  iflytekSkillHub: boolean;
}

export type TargetScope = 'global' | 'project';

export type TargetKind = 'agent' | 'custom';

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
}

export interface AgentPreset {
  id: string;
  displayName: string;
  globalPath: string;
  projectRelativePath?: string;
  iconUrl?: string;
}

export interface Target {
  id: string;
  name: string;
  scope: TargetScope;
  kind: TargetKind;
  agentId?: string;
  projectId?: string;
  customPath?: string;
  skillsDir: string;
  createdAt: string;
  updatedAt: string;
}

export interface Installation {
  id: string;
  skillDirName: string;
  skillName: string;
  sourcePath: string;
  targetId: string;
  linkPath: string;
  linkType: LinkType;
  createdAt: string;
  skillStorageKey: string;
}

export interface SkillRepo {
  host: string;
  provider: string;
  projectPath: string;
  owner: string;
  name: string;
  branch: string;
  enabled: boolean;
}

export interface SkillRecord {
  repoHost: string;
  projectPath: string;
  source: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  directory: string;
  contentHash: string;
  installedAt: string;
  storageKey: string;
  linkName: string;
  repoSlug: string;
  hubEndpointId: string;
  hubSkillGroup: string;
  hubSkillId: string;
  /** Hub/remote no longer has this skill; local copy may still exist. */
  sourceMissing?: boolean;
}

export interface DiscoverableSkill {
  key: string;
  name: string;
  description: string;
  directory: string;
  installDirName: string;
  repoHost: string;
  projectPath: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  source: string;
  storageKey: string;
  linkName: string;
  repoSlug: string;
  hubEndpointId: string;
  hubSkillGroup: string;
  hubSkillId: string;
}

export interface DiscoverSkillsResult {
  skills: DiscoverableSkill[];
  warnings: string[];
}

export interface SkillRepoChangeResult {
  repos: SkillRepo[];
  discoverSkills: DiscoverableSkill[];
}

export interface PreviewAddRepoResult {
  canSave: boolean;
  needsPat: boolean;
  host: string | null;
  provider: string | null;
  projectPath: string | null;
  branch: string | null;
  error: AppErrorDto | null;
}

export interface SkillUpdateInfo {
  dirName: string;
  name: string;
  currentHash?: string;
  remoteHash: string;
  storageKey: string;
}

export interface StartupSkillRefreshResult {
  discoverSkills: DiscoverableSkill[];
  pendingUpdates: SkillUpdateInfo[];
  warnings: string[];
}

export interface SkillDiscoverCache {
  fetchedAt: string | null;
  skills: DiscoverableSkill[];
}

export interface SkillUpdateCache {
  checkedAt: string | null;
  updates: SkillUpdateInfo[];
}

export interface SkillHubLocalState {
  skills: SkillView[];
  validCount: number;
  invalidCount: number;
  pendingUpdateCount: number;
  lastScanAt: string;
  skillRecords: Record<string, SkillRecord>;
}

export interface SmartPastePreview {
  name: string;
  description: string;
  installDirName: string;
  repoHost: string;
  projectPath: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  directory: string;
  source: string;
}

export interface UpdateAllSkillsResult {
  updated: string[];
  failed: { dirName: string; error: string }[];
}

export interface SkillHubEndpoint {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
}

export interface IflytekSkillHubEndpoint {
  id: string;
  name: string;
  baseUrl: string;
  enabled: boolean;
}

export interface SkillHubEndpointChangeResult {
  endpoints: SkillHubEndpoint[];
  discoverSkills: DiscoverableSkill[];
  /** Present on iFlytek CRUD results; optional so Skills Sync mutations stay compatible. */
  iflytekSkillHubEndpoints?: IflytekSkillHubEndpoint[];
}

export type SkillMarkdownRequest =
  | { kind: 'installed'; storageKey: string }
  | { kind: 'discover'; discoverKey: string };

export interface SkillMarkdownPreview {
  title: string;
  description: string;
  markdownBody: string;
  origin: 'mainLibrary' | 'repoCache' | 'remoteFile' | 'hubArchive';
}

export interface AppConfig {
  version: number;
  settings: Settings;
  projects: Project[];
  targets: Target[];
  installations: Installation[];
  skillRepos?: SkillRepo[];
  skillRecords?: Record<string, SkillRecord>;
  skillDiscoverCache?: SkillDiscoverCache;
  skillUpdateCache?: SkillUpdateCache;
  /** Hosts that have a stored GitLab PAT (from credential store; mirrored on config for UI). */
  gitlabCredentialHosts?: string[];
  skillHubEndpoints?: SkillHubEndpoint[];
  iflytekSkillHubEndpoints?: IflytekSkillHubEndpoint[];
}

export interface SkillView {
  dirName: string;
  name: string | null;
  description: string | null;
  path: string;
  valid: boolean;
  validationErrors: string[];
  storageKey: string;
  linkName: string;
  /** Hub skill: main-library hash differs from record.contentHash */
  localDirty?: boolean;
}

export interface SkillWithTargetState {
  skill: SkillView;
  state: SkillInstallState;
  message: string | null;
}

export interface MigrationReportDto {
  backedUpConfig: string;
  backedUpMain?: string | null;
  succeeded: string[];
  failed: string[];
  orphanLocals: string[];
  linksRepaired: number;
}

export interface AppState {
  config: AppConfig;
  skills: SkillView[];
  selectedTargetId: string | null;
  selectedTargetSkills: SkillWithTargetState[];
  lastMigrationReport?: MigrationReportDto | null;
  /** When false, client should keep previous skills via mergeAppState. Defaults to true. */
  skillsIncluded?: boolean;
  /** Soft warnings from the last force-cleanup operation. */
  cleanupWarnings?: string[];
}

export interface SyncInstallFailure {
  storageKey: string;
  label: string;
  error: string;
}

export interface SyncTargetInstallationsResponse {
  installed: number;
  skipped: number;
  failed: SyncInstallFailure[];
  state: AppState;
}

export const emptyV6DiscoverableFields = {
  storageKey: '',
  linkName: '',
  repoSlug: '',
  hubEndpointId: '',
  hubSkillGroup: '',
  hubSkillId: '',
} as const;

export const emptyV6SkillRecordFields = {
  storageKey: '',
  linkName: '',
  repoSlug: '',
  hubEndpointId: '',
  hubSkillGroup: '',
  hubSkillId: '',
  sourceMissing: false,
} as const;

export const emptyV6SkillViewFields = {
  storageKey: '',
  linkName: '',
  localDirty: false,
} as const;
