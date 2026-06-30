export type LinkStrategy = 'auto';

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
}

export interface Target {
  id: string;
  name: string;
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
}

export interface SkillRepo {
  owner: string;
  name: string;
  branch: string;
  enabled: boolean;
}

export interface SkillRecord {
  source: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  directory: string;
  contentHash: string;
  installedAt: string;
}

export interface DiscoverableSkill {
  key: string;
  name: string;
  description: string;
  directory: string;
  installDirName: string;
  repoOwner: string;
  repoName: string;
  repoBranch: string;
  source: string;
}

export interface SkillRepoChangeResult {
  repos: SkillRepo[];
  discoverSkills: DiscoverableSkill[];
}

export interface SkillUpdateInfo {
  dirName: string;
  name: string;
  currentHash?: string;
  remoteHash: string;
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

export interface AppConfig {
  version: number;
  settings: Settings;
  targets: Target[];
  installations: Installation[];
  skillRepos?: SkillRepo[];
  skillRecords?: Record<string, SkillRecord>;
  skillDiscoverCache?: SkillDiscoverCache;
  skillUpdateCache?: SkillUpdateCache;
}

export interface SkillView {
  dirName: string;
  name: string | null;
  description: string | null;
  path: string;
  valid: boolean;
  validationErrors: string[];
}

export interface SkillWithTargetState {
  skill: SkillView;
  state: SkillInstallState;
  message: string | null;
}

export interface AppState {
  config: AppConfig;
  skills: SkillView[];
  selectedTargetId: string | null;
  selectedTargetSkills: SkillWithTargetState[];
}
