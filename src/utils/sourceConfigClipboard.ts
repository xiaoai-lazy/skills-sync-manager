import type { SkillHubEndpoint, SkillRepo } from '../model/types';

export function formatHubSourceConfig(endpoint: SkillHubEndpoint): string {
  return [
    '【Skill Hub 来源配置】',
    `名称：${endpoint.name}`,
    `Base URL：${endpoint.baseUrl}`,
    '',
    '在「来源管理 → 添加来源 → Skill Hub」中填入以上内容。',
  ].join('\n');
}

function repoShareUrl(repo: SkillRepo): string {
  const path = repo.projectPath || `${repo.owner}/${repo.name}`;
  return `https://${repo.host}/${path}`;
}

export function formatRepoSourceConfig(repo: SkillRepo): string {
  const provider = repo.provider === 'gitlab' ? 'GitLab' : 'GitHub';
  const lines = [`【${provider} 来源配置】`, `仓库链接：${repoShareUrl(repo)}`];

  if (repo.provider === 'gitlab') {
    lines.push('说明：私有仓库需自行在「密钥管理」中配置 GitLab PAT。');
  }

  lines.push(
    '',
    `在「来源管理 → 添加来源 → ${provider}」中粘贴仓库链接，从 ${repo.branch} 分支拉取技能。`,
  );

  return lines.join('\n');
}

export async function copyTextToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.style.position = 'fixed';
  textarea.style.left = '-9999px';
  document.body.appendChild(textarea);
  textarea.select();
  const ok = document.execCommand('copy');
  document.body.removeChild(textarea);
  if (!ok) {
    throw new Error('无法写入剪贴板');
  }
}
