import { vi, describe, it, expect, beforeEach, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import SourceTree from '../components/skill-hub/SourceTree';
import { ALL_NODE_ID } from '../components/skill-hub/sourceTreeUtils';
import type { SkillHubEndpoint, SkillView } from '../model/types';
import { emptyV6SkillViewFields } from '../model/types';

const mockSkills: SkillView[] = [
  {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas.',
    path: 'C:\\skills\\brainstorming',
    valid: true,
    validationErrors: [],
    ...emptyV6SkillViewFields,
    storageKey: 'local/brainstorming',
    linkName: 'brainstorming',
  },
];

describe('SourceTree', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders source tree header and all node', () => {
    render(
      <SourceTree
        tab="installed"
        endpoints={[]}
        repos={[]}
        discoverSkills={[]}
        installedSkills={mockSkills}
        skillRecords={{}}
        selectedNodeId={ALL_NODE_ID}
        onSelectNode={vi.fn()}
      />,
    );

    expect(screen.getByText('来源')).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /全部/ })).toBeInTheDocument();
    expect(screen.getByText('本地导入')).toBeInTheDocument();
  });

  it('shows configured hub endpoints even with no hub-installed skills', () => {
    const endpoints: SkillHubEndpoint[] = [
      {
        id: 'company-hub',
        name: 'oxygen 团队 hub',
        baseUrl: 'http://127.0.0.1:3337',
        enabled: true,
      },
      {
        id: 'disabled-hub',
        name: '停用 Hub',
        baseUrl: 'http://127.0.0.1:3338',
        enabled: false,
      },
    ];

    render(
      <SourceTree
        tab="discover"
        endpoints={endpoints}
        repos={[]}
        discoverSkills={[]}
        installedSkills={[]}
        skillRecords={{}}
        selectedNodeId={ALL_NODE_ID}
        onSelectNode={vi.fn()}
      />,
    );

    expect(screen.getByRole('treeitem', { name: /Skills Sync Hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /oxygen 团队 hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /停用 Hub/ })).toBeInTheDocument();
  });

  it('nests Skills Sync and iFlytek under dual category roots', () => {
    const endpoints: SkillHubEndpoint[] = [
      {
        id: 'company-hub',
        name: 'oxygen 团队 hub',
        baseUrl: 'http://127.0.0.1:3337',
        enabled: true,
      },
    ];
    const iflytekEndpoints = [
      {
        id: 'xkw',
        name: '讯飞 Skill Hub',
        baseUrl: 'https://iflytek.example.com',
        enabled: true,
      },
    ];
    const discoverSkills = [
      {
        key: 'xkw:global/demo',
        name: 'demo',
        description: '',
        directory: 'global/demo',
        installDirName: 'demo',
        repoHost: '',
        projectPath: '',
        repoOwner: '',
        repoName: '',
        repoBranch: '',
        source: 'iflytek',
        storageKey: 'hub/xkw/global/demo',
        linkName: 'demo',
        repoSlug: '',
        hubEndpointId: 'xkw',
        hubSkillGroup: 'global',
        hubSkillId: 'demo',
      },
    ];

    render(
      <SourceTree
        tab="discover"
        endpoints={endpoints}
        iflytekEndpoints={iflytekEndpoints}
        repos={[]}
        discoverSkills={discoverSkills}
        installedSkills={[]}
        skillRecords={{}}
        selectedNodeId={ALL_NODE_ID}
        onSelectNode={vi.fn()}
      />,
    );

    expect(screen.getByRole('treeitem', { name: /Skills Sync Hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /iFlytek Skill Hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /oxygen 团队 hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /讯飞 Skill Hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /^global$/ })).toBeInTheDocument();
    expect(screen.queryByText(/ued/)).not.toBeInTheDocument();
  });

  it('renders nodeCountLabel after tree labels when provided', () => {
    render(
      <SourceTree
        tab="installed"
        endpoints={[]}
        repos={[]}
        discoverSkills={[]}
        installedSkills={mockSkills}
        skillRecords={{}}
        selectedNodeId={ALL_NODE_ID}
        onSelectNode={vi.fn()}
        nodeCountLabel={(nodeId) => (nodeId === ALL_NODE_ID ? '1/2' : null)}
      />,
    );

    const allNode = screen.getByRole('treeitem', { name: /全部/ });
    expect(allNode.querySelector('.tree-count')).toHaveTextContent('1/2');
    expect(screen.getByRole('treeitem', { name: /本地导入/ }).querySelector('.tree-count')).toBeNull();
  });
});
