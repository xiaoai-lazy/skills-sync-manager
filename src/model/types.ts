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

export interface AppConfig {
  version: number;
  settings: Settings;
  targets: Target[];
  installations: Installation[];
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
