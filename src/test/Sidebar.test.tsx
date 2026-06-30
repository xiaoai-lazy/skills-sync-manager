import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import type { ComponentProps } from 'react';
import Sidebar from '../components/Sidebar';
import type { Target } from '../model/types';

const sampleTargets: Target[] = [
  {
    id: 'target_1',
    name: 'Claude Global',
    skillsDir: '/tmp/target',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
  {
    id: 'target_2',
    name: 'Claude Project',
    skillsDir: '/tmp/target2',
    createdAt: '2026-06-23T00:00:00Z',
    updatedAt: '2026-06-23T00:00:00Z',
  },
];

function renderSidebar(overrides: Partial<ComponentProps<typeof Sidebar>> = {}) {
  const defaults = {
    targets: sampleTargets,
    selectedTargetId: 'target_1',
    mainView: 'skill-hub' as const,
    onOpenSkillHub: vi.fn(),
    onSelectTarget: vi.fn(),
    onAddTarget: vi.fn(),
    onEditTarget: vi.fn(),
    onDeleteTarget: vi.fn(),
  };
  return render(<Sidebar {...defaults} {...overrides} />);
}

describe('Sidebar', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders brand and Skill 中心 nav', () => {
    renderSidebar();

    expect(screen.getByText('Skills Sync')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /skill 中心/i })).toBeInTheDocument();
    expect(screen.getByText('目标目录')).toBeInTheDocument();
  });

  it('marks Skill 中心 nav active when mainView is skill-hub', () => {
    renderSidebar({ mainView: 'skill-hub' });

    const navButton = screen.getByRole('button', { name: /skill 中心/i });
    expect(navButton).toHaveClass('active');
  });

  it('marks target selected only when mainView is target', () => {
    const { rerender } = renderSidebar({
      mainView: 'skill-hub',
      selectedTargetId: 'target_1',
    });

    let selectedItems = document.querySelectorAll('.target-item.selected');
    expect(selectedItems).toHaveLength(0);

    rerender(
      <Sidebar
        targets={sampleTargets}
        selectedTargetId="target_1"
        mainView="target"
        onOpenSkillHub={vi.fn()}
        onSelectTarget={vi.fn()}
        onAddTarget={vi.fn()}
        onEditTarget={vi.fn()}
        onDeleteTarget={vi.fn()}
      />
    );

    selectedItems = document.querySelectorAll('.target-item.selected');
    expect(selectedItems).toHaveLength(1);
    expect(selectedItems[0]).toHaveTextContent('Claude Global');
  });

  it('calls onOpenSkillHub when Skill 中心 nav clicked', async () => {
    const onOpenSkillHub = vi.fn();
    renderSidebar({ mainView: 'target', onOpenSkillHub });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /skill 中心/i }));

    expect(onOpenSkillHub).toHaveBeenCalledTimes(1);
  });

  it('calls onSelectTarget when a target is clicked', async () => {
    const onSelectTarget = vi.fn();
    renderSidebar({ onSelectTarget });

    const user = userEvent.setup();
    await user.click(screen.getByText('Claude Project'));

    expect(onSelectTarget).toHaveBeenCalledWith('target_2');
  });

  it('calls onAddTarget when add target button clicked', async () => {
    const onAddTarget = vi.fn();
    renderSidebar({ onAddTarget });

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /add target/i }));

    expect(onAddTarget).toHaveBeenCalledTimes(1);
  });

  it('renders empty state when no targets', () => {
    renderSidebar({ targets: [] });

    expect(screen.getByText('暂无目标目录')).toBeInTheDocument();
  });
});
