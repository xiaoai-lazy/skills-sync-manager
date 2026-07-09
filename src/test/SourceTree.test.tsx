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

    expect(screen.getByRole('treeitem', { name: /oxygen 团队 hub/ })).toBeInTheDocument();
    expect(screen.getByRole('treeitem', { name: /停用 Hub/ })).toBeInTheDocument();
  });
});
