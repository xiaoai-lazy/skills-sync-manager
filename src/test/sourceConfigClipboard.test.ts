import { describe, expect, it } from 'vitest';
import {
  formatHubSourceConfig,
  formatIflytekHubSourceConfig,
  formatRepoSourceConfig,
} from '../utils/sourceConfigClipboard';
import type { SkillHubEndpoint, SkillRepo } from '../model/types';

const hubEndpoint: SkillHubEndpoint = {
  id: 'hub-1',
  name: 'oxygen 团队 hub',
  baseUrl: 'http://127.0.0.1:3337',
  enabled: true,
};

const githubRepo: SkillRepo = {
  host: 'github.com',
  provider: 'github',
  projectPath: 'obra/superpowers',
  owner: 'obra',
  name: 'superpowers',
  branch: 'main',
  enabled: true,
};

const gitlabRepo: SkillRepo = {
  host: 'gitlab.example.com',
  provider: 'gitlab',
  projectPath: 'acme/tools',
  owner: 'acme',
  name: 'tools',
  branch: 'master',
  enabled: true,
};

describe('sourceConfigClipboard', () => {
  it('formats Skills Sync Hub source config with name and base URL', () => {
    const text = formatHubSourceConfig(hubEndpoint);
    expect(text).toContain('【Skills Sync Hub 来源配置】');
    expect(text).toContain('名称：oxygen 团队 hub');
    expect(text).toContain('Base URL：http://127.0.0.1:3337');
    expect(text).toContain('Skills Sync Hub');
  });

  it('formats iFlytek Skill Hub source config with name and base URL', () => {
    const text = formatIflytekHubSourceConfig({
      id: 'xkw',
      name: '讯飞 Skill Hub',
      baseUrl: 'https://iflytek.example.com',
      enabled: true,
    });
    expect(text).toContain('【iFlytek Skill Hub 来源配置】');
    expect(text).toContain('名称：讯飞 Skill Hub');
    expect(text).toContain('Base URL：https://iflytek.example.com');
    expect(text).toContain('iFlytek Skill Hub');
  });

  it('formats github repo source config', () => {
    const text = formatRepoSourceConfig(githubRepo);
    expect(text).toContain('【GitHub 来源配置】');
    expect(text).toContain('仓库链接：https://github.com/obra/superpowers');
    expect(text).toContain('从 main 分支拉取技能');
    expect(text).not.toContain('分支：main');
    expect(text).not.toContain('PAT');
  });

  it('formats gitlab repo source config with PAT hint', () => {
    const text = formatRepoSourceConfig(gitlabRepo);
    expect(text).toContain('【GitLab 来源配置】');
    expect(text).toContain('仓库链接：https://gitlab.example.com/acme/tools');
    expect(text).toContain('从 master 分支拉取技能');
    expect(text).not.toContain('分支：master');
    expect(text).toContain('GitLab PAT');
  });
});
