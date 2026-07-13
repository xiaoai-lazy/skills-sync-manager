import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen, within } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import KeysManageDialog from '../components/skill-hub/KeysManageDialog';
import type { SkillRepo } from '../model/types';

const gitlabRepo = (host: string, projectPath: string): SkillRepo => ({
  host,
  provider: 'gitlab',
  projectPath,
  owner: projectPath.split('/')[0],
  name: projectPath.split('/').at(-1) ?? projectPath,
  branch: 'main',
  enabled: true,
});

const githubRepo: SkillRepo = {
  host: 'github.com',
  provider: 'github',
  projectPath: 'acme/public-skills',
  owner: 'acme',
  name: 'public-skills',
  branch: 'main',
  enabled: true,
};

function renderDialog(repos: SkillRepo[], configuredHosts: string[] = []) {
  return render(
    <KeysManageDialog
      open
      repos={repos}
      configuredHosts={configuredHosts}
      nestedDialogOpen={false}
      onClose={vi.fn()}
      onAuthenticate={vi.fn()}
      onUpdate={vi.fn()}
      onRemove={vi.fn().mockResolvedValue(undefined)}
    />,
  );
}

afterEach(cleanup);

describe('KeysManageDialog', () => {
  it('groups GitLab repositories by normalized host and excludes GitHub', () => {
    renderDialog([
      gitlabRepo('GitLab.Example.COM', 'team/skills'),
      gitlabRepo('gitlab.example.com', 'team/docs'),
      gitlabRepo('gitlab.internal', 'platform/tools'),
      githubRepo,
    ]);

    const exampleGroup = screen.getByTestId('gitlab-host-gitlab.example.com');
    expect(within(exampleGroup).getByText('team/skills')).toBeInTheDocument();
    expect(within(exampleGroup).getByText('team/docs')).toBeInTheDocument();
    expect(within(exampleGroup).getByText('2 个仓库')).toBeInTheDocument();
    expect(screen.getByTestId('gitlab-host-gitlab.internal')).toHaveTextContent(
      'platform/tools',
    );
    expect(screen.queryByText('acme/public-skills')).not.toBeInTheDocument();
  });

  it('shows an empty state when no GitLab repositories are configured', () => {
    renderDialog([githubRepo]);
    expect(screen.getByText('暂无已添加的 GitLab 来源仓库')).toBeInTheDocument();
  });
});
