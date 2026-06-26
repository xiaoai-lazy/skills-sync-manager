# Main Library 独立删除页面实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 skill 删除功能从 Target 详情页移除，新增独立的 Main Library 详情页用于查看和删除所有 skill；应用启动后默认显示 Main Library 详情页。

**Architecture:** 在 `App.tsx` 中新增 `mainView` 状态（`'main-library' | 'target'`）控制主面板渲染；将现有 `MainLibraryPanel` 拆分为 `MainLibrarySummary`（Sidebar 摘要）和 `MainLibraryPage`（主面板详情）；复用现有的 `deleteMainSkill` command 与 `ConfirmDialog`。

**Tech Stack:** React 18 + TypeScript + Vite + Vitest + React Testing Library + Tauri v2

---

## 文件结构

- **新建** `src/components/MainLibrarySummary.tsx`
  - Sidebar 中 Main Library 的摘要卡片：显示目录路径、valid/invalid 数量、「Manage Skills」入口。
- **新建** `src/components/MainLibraryPage.tsx`
  - 主面板中的 Main Library 详情页：显示所有 skill 列表，每项带删除按钮。
- **新建** `src/test/MainLibrarySummary.test.tsx`
  - `MainLibrarySummary` 的单元测试。
- **新建** `src/test/MainLibraryPage.test.tsx`
  - `MainLibraryPage` 的单元测试。
- **修改** `src/components/Sidebar.tsx`
  - 用 `MainLibrarySummary` 替换 `MainLibraryPanel`；新增 `onManageSkills` prop。
- **修改** `src/components/TargetDetail.tsx`
  - 移除未使用的 `onDeleteMainSkill` prop。
- **修改** `src/App.tsx`
  - 新增 `mainView` 状态，默认 `'main-library'`；主面板条件渲染；Sidebar 透传 `onManageSkills`。
- **修改** `src/test/app.test.tsx`
  - 更新现有测试以匹配新的默认视图与交互流程。
- **删除** `src/components/MainLibraryPanel.tsx`
  - 功能已被拆分到 `MainLibrarySummary` 与 `MainLibraryPage`。
- **修改** `src/styles.css`
  - 为 `MainLibraryPage` 与新的「Manage Skills」按钮添加/调整样式。

---

## Task 1: 创建 MainLibrarySummary 组件

**Files:**
- Create: `src/test/MainLibrarySummary.test.tsx`
- Create: `src/components/MainLibrarySummary.tsx`

- [ ] **Step 1: Write the failing test**

创建 `src/test/MainLibrarySummary.test.tsx`：

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import MainLibrarySummary from '../components/MainLibrarySummary';

describe('MainLibrarySummary', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders directory path and skill counts', () => {
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={2}
        onSetMainSkillsDir={vi.fn()}
        onManageSkills={vi.fn()}
      />
    );

    expect(screen.getByText('/tmp/main-skills')).toBeInTheDocument();
    expect(screen.getByText('10 valid')).toBeInTheDocument();
    expect(screen.getByText('2 invalid')).toBeInTheDocument();
  });

  it('calls onManageSkills when manage button clicked', async () => {
    const onManageSkills = vi.fn();
    render(
      <MainLibrarySummary
        mainSkillsDir="/tmp/main-skills"
        validSkillCount={10}
        invalidSkillCount={2}
        onSetMainSkillsDir={vi.fn()}
        onManageSkills={onManageSkills}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /manage skills/i }));

    expect(onManageSkills).toHaveBeenCalledTimes(1);
  });

  it('calls onSetMainSkillsDir when set directory button clicked', async () => {
    const onSetMainSkillsDir = vi.fn();
    render(
      <MainLibrarySummary
        mainSkillsDir={null}
        validSkillCount={0}
        invalidSkillCount={0}
        onSetMainSkillsDir={onSetMainSkillsDir}
        onManageSkills={vi.fn()}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: /set main directory/i }));

    expect(onSetMainSkillsDir).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test -- src/test/MainLibrarySummary.test.tsx
```

Expected: FAIL with "Cannot find module '../components/MainLibrarySummary'" or similar.

- [ ] **Step 3: Write minimal implementation**

创建 `src/components/MainLibrarySummary.tsx`：

```tsx
import React from 'react';

export interface MainLibrarySummaryProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  onSetMainSkillsDir: () => void;
  onManageSkills: () => void;
}

function MainLibrarySummary(props: MainLibrarySummaryProps) {
  return (
    <section className="main-library-panel">
      <h2>Main Library</h2>
      {props.mainSkillsDir ? (
        <div className="dir-info">
          <div className="dir-path" title={props.mainSkillsDir}>
            {props.mainSkillsDir}
          </div>
          <div className="skill-counts">
            <span className="count valid">{props.validSkillCount} valid</span>
            {props.invalidSkillCount > 0 && (
              <span className="count invalid">
                {props.invalidSkillCount} invalid
              </span>
            )}
          </div>
        </div>
      ) : (
        <div className="empty-state">
          <p>No main skills directory configured.</p>
          <button onClick={props.onSetMainSkillsDir}>Set Main Directory</button>
        </div>
      )}
      {props.mainSkillsDir && (
        <>
          <button className="secondary-button" onClick={props.onSetMainSkillsDir}>
            Change Directory
          </button>
          <button className="secondary-button" onClick={props.onManageSkills}>
            Manage Skills
          </button>
        </>
      )}
    </section>
  );
}

export default MainLibrarySummary;
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
npm test -- src/test/MainLibrarySummary.test.tsx
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/MainLibrarySummary.tsx src/test/MainLibrarySummary.test.tsx
git commit -m "feat: add MainLibrarySummary component for sidebar

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 2: 创建 MainLibraryPage 组件

**Files:**
- Create: `src/test/MainLibraryPage.test.tsx`
- Create: `src/components/MainLibraryPage.tsx`

- [ ] **Step 1: Write the failing test**

创建 `src/test/MainLibraryPage.test.tsx`：

```tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom/vitest';
import MainLibraryPage from '../components/MainLibraryPage';
import type { SkillView } from '../model/types';

const mockSkills: SkillView[] = [
  {
    dirName: 'brainstorming',
    name: 'brainstorming',
    description: 'Explore ideas.',
    path: '/tmp/main-skills/brainstorming',
    valid: true,
    validationErrors: [],
  },
  {
    dirName: 'invalid-skill',
    name: null,
    description: null,
    path: '/tmp/main-skills/invalid-skill',
    valid: false,
    validationErrors: ['Missing skill.yaml'],
  },
];

describe('MainLibraryPage', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders all skills with delete buttons', () => {
    render(
      <MainLibraryPage
        skills={mockSkills}
        validSkillCount={1}
        invalidSkillCount={1}
        onDeleteMainSkill={vi.fn()}
      />
    );

    expect(screen.getByRole('heading', { name: /all skills/i })).toBeInTheDocument();
    expect(screen.getByText('brainstorming')).toBeInTheDocument();
    expect(screen.getByText('invalid-skill')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /delete/i })).toHaveLength(2);
  });

  it('calls onDeleteMainSkill with skill dir name when delete button clicked', async () => {
    const onDeleteMainSkill = vi.fn();
    render(
      <MainLibraryPage
        skills={mockSkills}
        validSkillCount={1}
        invalidSkillCount={1}
        onDeleteMainSkill={onDeleteMainSkill}
      />
    );

    const user = userEvent.setup();
    await user.click(screen.getAllByRole('button', { name: /delete/i })[0]);

    expect(onDeleteMainSkill).toHaveBeenCalledWith('brainstorming');
  });

  it('renders empty state when no skills', () => {
    render(
      <MainLibraryPage
        skills={[]}
        validSkillCount={0}
        invalidSkillCount={0}
        onDeleteMainSkill={vi.fn()}
      />
    );

    expect(screen.getByText('No skills found in the main directory.')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test -- src/test/MainLibraryPage.test.tsx
```

Expected: FAIL with module not found.

- [ ] **Step 3: Write minimal implementation**

创建 `src/components/MainLibraryPage.tsx`：

```tsx
import React from 'react';
import type { SkillView } from '../model/types';

export interface MainLibraryPageProps {
  skills: SkillView[];
  validSkillCount: number;
  invalidSkillCount: number;
  onDeleteMainSkill: (skillDirName: string) => void;
}

function MainLibraryPage(props: MainLibraryPageProps) {
  return (
    <section className="main-library-page">
      <h2>Main Library</h2>
      <div className="skill-counts" style={{ marginBottom: '1rem' }}>
        <span className="count valid">{props.validSkillCount} valid</span>
        {props.invalidSkillCount > 0 && (
          <span className="count invalid">{props.invalidSkillCount} invalid</span>
        )}
      </div>

      <div className="library-skill-list">
        <h3>All Skills ({props.skills.length})</h3>
        {props.skills.length === 0 ? (
          <div className="empty-state">
            <p>No skills found in the main directory.</p>
          </div>
        ) : (
          <ul className="skill-list">
            {props.skills.map((skill) => (
              <li key={skill.dirName}>
                <div className={`skill-row ${!skill.valid ? 'invalid' : ''}`}>
                  <div className="skill-info">
                    <div className="skill-name">{skill.name ?? skill.dirName}</div>
                    {skill.description && (
                      <div className="skill-description">{skill.description}</div>
                    )}
                    <div className="skill-dir">{skill.dirName}</div>
                    {!skill.valid && skill.validationErrors.length > 0 && (
                      <div className="skill-message">
                        {skill.validationErrors.join(', ')}
                      </div>
                    )}
                  </div>
                  <div className="skill-actions">
                    <button
                      className="danger-button"
                      onClick={() => props.onDeleteMainSkill(skill.dirName)}
                      aria-label={`Delete skill ${skill.dirName}`}
                      title="从主库删除"
                    >
                      删除
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

export default MainLibraryPage;
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
npm test -- src/test/MainLibraryPage.test.tsx
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/MainLibraryPage.tsx src/test/MainLibraryPage.test.tsx
git commit -m "feat: add MainLibraryPage component for main panel

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 3: 更新 Sidebar 使用 MainLibrarySummary

**Files:**
- Modify: `src/components/Sidebar.tsx`

- [ ] **Step 1: Write the failing test**

修改 `src/test/app.test.tsx` 中现有用例 `renders app title and main directory section`，验证「Manage Skills」按钮存在：

```tsx
it('renders app title, main directory section and manage skills button', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);
  render(<App />);

  expect(await screen.findByRole('heading', { name: 'Main Library' })).toBeInTheDocument();
  expect(await screen.findByText('/tmp/main-skills')).toBeInTheDocument();
  expect(await screen.findByRole('button', { name: /manage skills/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test -- src/test/app.test.tsx -t "renders app title"
```

Expected: FAIL — "Manage Skills" button not found.

- [ ] **Step 3: Write minimal implementation**

修改 `src/components/Sidebar.tsx`：

```tsx
import React from 'react';
import type { Target } from '../model/types';
import MainLibrarySummary from './MainLibrarySummary';
import TargetList from './TargetList';

export interface SidebarProps {
  mainSkillsDir: string | null;
  validSkillCount: number;
  invalidSkillCount: number;
  targets: Target[];
  selectedTargetId: string | null;
  onSelectTarget: (targetId: string) => void;
  onAddTarget: () => void;
  onEditTarget: (target: Target) => void;
  onDeleteTarget: (target: Target) => void;
  onSetMainSkillsDir: () => void;
  onManageSkills: () => void;
}

function Sidebar(props: SidebarProps) {
  return (
    <aside className="sidebar">
      <MainLibrarySummary
        mainSkillsDir={props.mainSkillsDir}
        validSkillCount={props.validSkillCount}
        invalidSkillCount={props.invalidSkillCount}
        onSetMainSkillsDir={props.onSetMainSkillsDir}
        onManageSkills={props.onManageSkills}
      />
      <TargetList
        targets={props.targets}
        selectedTargetId={props.selectedTargetId}
        onSelectTarget={props.onSelectTarget}
        onAddTarget={props.onAddTarget}
        onEditTarget={props.onEditTarget}
        onDeleteTarget={props.onDeleteTarget}
      />
    </aside>
  );
}

export default Sidebar;
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
npm test -- src/test/app.test.tsx -t "renders app title"
```

Expected: 该用例仍可能 FAIL，因为 `App.tsx` 还未传递 `onManageSkills`。此时 TypeScript 会报错导致测试无法运行，先进入 Task 5 完成 App.tsx 后一起验证。

> **注意：** 由于 `SidebarProps` 新增必填 prop，单独更新 `Sidebar.tsx` 会导致 `App.tsx` TypeScript 编译失败。建议将 Task 3 和 Task 5 连续执行，或在 Task 5 完成后再回过来运行此测试。

- [ ] **Step 5: Commit**

```bash
git add src/components/Sidebar.tsx
git commit -m "refactor: Sidebar uses MainLibrarySummary

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 4: 更新 TargetDetail 移除 onDeleteMainSkill

**Files:**
- Modify: `src/components/TargetDetail.tsx`

- [ ] **Step 1: Write the failing test**

在 `src/test/app.test.tsx` 中新增用例，验证 Target 详情中没有删除按钮（当前代码中 `TargetDetail` 已不使用删除按钮，但保留 prop；测试确保删除按钮不出现）：

```tsx
it('target detail does not show delete skill button', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);
  render(<App />);

  // First select a target from the sidebar
  const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
  const user = userEvent.setup();
  await user.click(targetList!.querySelector('.target-name')!);

  await waitFor(() => {
    expect(screen.getByRole('heading', { name: 'Claude Global' })).toBeInTheDocument();
  });

  expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
npm test -- src/test/app.test.tsx -t "target detail does not show"
```

Expected: 在 Task 5 完成前，该测试因默认视图尚未切换而失败；在 Task 5 完成后应 PASS。

- [ ] **Step 3: Write minimal implementation**

修改 `src/components/TargetDetail.tsx`，移除 `onDeleteMainSkill` prop：

```tsx
import React from 'react';
import type { Target, SkillWithTargetState } from '../model/types';
import SkillRow from './SkillRow';

export interface TargetDetailProps {
  target: Target | null;
  skills: SkillWithTargetState[];
  pendingSkillKey: string | null;
  onToggleSkill: (skillDirName: string, state: import('../model/types').SkillInstallState) => void;
}

function TargetDetail(props: TargetDetailProps) {
  if (!props.target) {
    return (
      <div className="target-detail empty">
        <h2>No Target Selected</h2>
        <p>Select a target from the sidebar to view and manage its skills.</p>
      </div>
    );
  }

  const validSkills = props.skills.filter((s) => s.skill.valid);
  const invalidSkills = props.skills.filter((s) => !s.skill.valid);

  return (
    <div className="target-detail">
      <header className="target-detail-header">
        <h2>{props.target.name}</h2>
        <div className="target-meta" title={props.target.skillsDir}>
          {props.target.skillsDir}
        </div>
      </header>

      <section className="skill-section">
        <h3>Skills ({validSkills.length})</h3>
        {validSkills.length === 0 ? (
          <div className="empty-state">
            <p>No valid skills found in the main library.</p>
          </div>
        ) : (
          <ul className="skill-list">
            {validSkills.map((item) => (
              <li key={item.skill.dirName}>
                <SkillRow
                  item={item}
                  pending={props.pendingSkillKey === item.skill.dirName}
                  onToggle={props.onToggleSkill}
                />
              </li>
            ))}
          </ul>
        )}
      </section>

      {invalidSkills.length > 0 && (
        <section className="skill-section invalid-section">
          <h3>Invalid Skills ({invalidSkills.length})</h3>
          <ul className="skill-list">
            {invalidSkills.map((item) => (
              <li key={item.skill.dirName}>
                <SkillRow
                  item={item}
                  pending={props.pendingSkillKey === item.skill.dirName}
                  onToggle={props.onToggleSkill}
                />
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export default TargetDetail;
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
npm test -- src/test/app.test.tsx -t "target detail does not show"
```

Expected: PASS after Task 5 完成。

- [ ] **Step 5: Commit**

```bash
git add src/components/TargetDetail.tsx
git commit -m "refactor: remove delete skill prop from TargetDetail

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 5: 更新 App.tsx 添加 mainView 状态并条件渲染

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/test/app.test.tsx`

- [ ] **Step 1: Write the failing tests**

更新 `src/test/app.test.tsx`。由于默认视图改为 Main Library，原有测试需要调整。下面是完整的更新后相关用例（保留未列出的用例不变）：

```tsx
// 替换原有 "renders app title and main directory section"
it('renders main library page by default', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);
  render(<App />);

  expect(await screen.findByRole('heading', { name: 'Main Library' })).toBeInTheDocument();
  expect(await screen.findByRole('heading', { name: /all skills/i })).toBeInTheDocument();
  expect(await screen.findByText('/tmp/main-skills')).toBeInTheDocument();
  expect(await screen.findByRole('button', { name: /manage skills/i })).toBeInTheDocument();
});

// 替换原有 "selecting a target shows its skill rows"
it('selecting a target from sidebar switches to target detail', async () => {
  const twoTargetState = withTwoTargets(baseAppState);
  vi.mocked(getAppState).mockResolvedValue(twoTargetState);
  render(<App />);

  // Wait for Main Library to render first
  await screen.findByRole('heading', { name: /all skills/i });

  const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
  expect(targetList).toBeInTheDocument();
  const targetItems = targetList!.querySelectorAll('.target-name');
  expect(targetItems.length).toBe(2);

  const user = userEvent.setup();
  await user.click(targetItems[1]!);

  await waitFor(() => {
    expect(screen.getByRole('heading', { name: 'Claude Project' })).toBeInTheDocument();
  });
  expect(screen.getByText('No valid skills found in the main library.')).toBeInTheDocument();
});

// 新增：点击 Manage Skills 保持在 Main Library 视图
it('clicking manage skills keeps main library view', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);
  render(<App />);

  await screen.findByRole('heading', { name: /all skills/i });

  const user = userEvent.setup();
  await user.click(await screen.findByRole('button', { name: /manage skills/i }));

  expect(screen.getByRole('heading', { name: /all skills/i })).toBeInTheDocument();
});

// 新增：Target 详情中没有删除按钮
it('target detail does not show delete skill button', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);
  render(<App />);

  const targetList = (await screen.findByRole('heading', { name: 'Targets' })).closest('section');
  const user = userEvent.setup();
  await user.click(targetList!.querySelector('.target-name')!);

  await waitFor(() => {
    expect(screen.getByRole('heading', { name: 'Claude Global' })).toBeInTheDocument();
  });

  expect(screen.queryByRole('button', { name: 'Delete' })).not.toBeInTheDocument();
});

// 更新原有 "delete skill button opens confirmation dialog"
it('delete skill button in main library opens confirmation dialog', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);

  render(<App />);
  await screen.findByRole('heading', { name: /all skills/i });

  const deleteButton = screen.getByRole('button', { name: 'Delete' });
  const user = userEvent.setup();
  await user.click(deleteButton);

  expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
  expect(screen.getByText(/brainstorming.*will be permanently deleted/)).toBeInTheDocument();
});

// 更新原有 "canceling confirmation does not call delete command"
it('canceling confirmation in main library does not call delete command', async () => {
  vi.mocked(getAppState).mockResolvedValue(baseAppState);

  render(<App />);
  await screen.findByRole('heading', { name: /all skills/i });

  const deleteButton = screen.getByRole('button', { name: 'Delete' });
  const user = userEvent.setup();
  await user.click(deleteButton);

  expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();

  const cancelButton = screen.getByRole('button', { name: /cancel/i });
  await user.click(cancelButton);

  await waitFor(() => {
    expect(screen.queryByText('Confirm Deletion')).not.toBeInTheDocument();
  });

  expect(deleteMainSkill).not.toHaveBeenCalled();
});

// 更新原有 "confirming deletion calls delete command with confirmed = true"
it('confirming deletion in main library calls delete command with confirmed = true', async () => {
  const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
  vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

  const afterDeleteState = {
    ...baseAppState,
    skills: [],
    selectedTargetSkills: [],
  };
  vi.mocked(deleteMainSkill).mockResolvedValue(afterDeleteState);

  render(<App />);
  await screen.findByRole('heading', { name: /all skills/i });

  const deleteButton = screen.getByRole('button', { name: 'Delete' });
  const user = userEvent.setup();
  await user.click(deleteButton);

  expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();

  const dialog = screen.getByRole('dialog');
  const confirmButton = dialog.querySelector('.danger-button') as HTMLElement;
  await user.click(confirmButton);

  await waitFor(() => {
    expect(deleteMainSkill).toHaveBeenCalledWith('brainstorming', true);
  });
});

// 更新原有 "delete dialog shows link count when skill has installations"
it('delete dialog in main library shows link count when skill has installations', async () => {
  const stateWithInstallations = withInstallations(baseAppState, 'brainstorming');
  vi.mocked(getAppState).mockResolvedValue(stateWithInstallations);

  render(<App />);
  await screen.findByRole('heading', { name: /all skills/i });

  const deleteButton = screen.getByRole('button', { name: 'Delete' });
  const user = userEvent.setup();
  await user.click(deleteButton);

  expect(await screen.findByText('Confirm Deletion')).toBeInTheDocument();
  expect(screen.getByText(/1 recorded target link\(s\) will be removed/)).toBeInTheDocument();
});

// 更新原有 "invalid skills are rendered in invalid section"
it('invalid skills are rendered in main library list', async () => {
  const stateWithInvalid = withInvalidSkill(baseAppState);
  vi.mocked(getAppState).mockResolvedValue(stateWithInvalid);

  render(<App />);
  await screen.findByRole('heading', { name: /all skills/i });

  expect(screen.getByText('invalid-skill')).toBeInTheDocument();
  expect(screen.getByText('Missing skill.yaml')).toBeInTheDocument();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
npm test -- src/test/app.test.tsx
```

Expected: Multiple FAIL — Main Library page not found, delete button not found in Main Library, etc.

- [ ] **Step 3: Write minimal implementation**

修改 `src/App.tsx`：

```tsx
import React, { useState, useEffect, useCallback } from 'react';
import type { AppState, Target, SkillWithTargetState, SkillInstallState } from './model/types';
import {
  getAppState,
  setMainSkillsDir,
  addTarget,
  updateTarget,
  deleteTarget,
  installSkill,
  uninstallSkill,
  deleteMainSkill,
} from './api/commands';
import Sidebar from './components/Sidebar';
import TargetDetail from './components/TargetDetail';
import MainLibraryPage from './components/MainLibraryPage';
import ConfirmDialog from './components/ConfirmDialog';

type MainView = 'main-library' | 'target';

function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return '操作失败，请查看日志或重试。';
}

function App() {
  const [appState, setAppState] = useState<AppState | null>(null);
  const [selectedTargetId, setSelectedTargetId] = useState<string | null>(null);
  const [mainView, setMainView] = useState<MainView>('main-library');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pendingSkillKey, setPendingSkillKey] = useState<string | null>(null);
  const [deleteSkillDirName, setDeleteSkillDirName] = useState<string | null>(null);

  const refresh = useCallback(
    async (nextSelectedTargetId: string | null = selectedTargetId): Promise<void> => {
      setLoading(true);
      try {
        const next = await getAppState(nextSelectedTargetId);
        setAppState(next);
        setSelectedTargetId(next.selectedTargetId);
        setError(null);
      } catch (err) {
        setError(errorMessage(err));
      } finally {
        setLoading(false);
      }
    },
    [selectedTargetId]
  );

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSetMainSkillsDir = async () => {
    if (!appState) return;
    const path = window.prompt(
      'Enter main skills directory path:',
      appState.config.settings.mainSkillsDir ?? ''
    );
    if (path === null) return;
    setPendingSkillKey('mainDir');
    try {
      const next = await setMainSkillsDir(path);
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleAddTarget = async () => {
    const name = window.prompt('Enter target name:');
    if (name === null) return;
    const skillsDir = window.prompt('Enter target skills directory path:');
    if (skillsDir === null) return;
    if (!name.trim() || !skillsDir.trim()) return;
    setPendingSkillKey('addTarget');
    try {
      const next = await addTarget(name.trim(), skillsDir.trim());
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setMainView('target');
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleEditTarget = async (target: Target) => {
    const name = window.prompt('Enter new target name:', target.name);
    if (name === null) return;
    const skillsDir = window.prompt('Enter new target skills directory path:', target.skillsDir);
    if (skillsDir === null) return;
    if (!name.trim() || !skillsDir.trim()) return;
    setPendingSkillKey(`edit-${target.id}`);
    try {
      const next = await updateTarget(target.id, name.trim(), skillsDir.trim());
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleDeleteTarget = async (target: Target) => {
    if (!window.confirm(`Delete target "${target.name}"?`)) return;
    setPendingSkillKey(`delete-${target.id}`);
    try {
      const next = await deleteTarget(target.id, false);
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      const msg = errorMessage(err);
      if (
        window.confirm(
          'Target has recorded installations. Remove links and delete target?'
        )
      ) {
        try {
          const next = await deleteTarget(target.id, true);
          setAppState(next);
          setSelectedTargetId(next.selectedTargetId);
          setError(null);
        } catch (err2) {
          setError(errorMessage(err2));
        }
      } else {
        setError(msg);
      }
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleSelectTarget = (targetId: string) => {
    setMainView('target');
    setSelectedTargetId(targetId);
    refresh(targetId);
  };

  const handleManageSkills = () => {
    setMainView('main-library');
  };

  const handleToggleSkill = async (skillDirName: string, state: SkillInstallState) => {
    if (!appState || !selectedTargetId) return;
    setPendingSkillKey(skillDirName);
    try {
      const next =
        state === 'notInstalled'
          ? await installSkill(selectedTargetId, skillDirName)
          : await uninstallSkill(selectedTargetId, skillDirName);
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleDeleteMainSkill = (skillDirName: string) => {
    setDeleteSkillDirName(skillDirName);
  };

  const handleConfirmDeleteMainSkill = async () => {
    if (!deleteSkillDirName || !appState) return;
    setPendingSkillKey(`delete-skill-${deleteSkillDirName}`);
    setDeleteSkillDirName(null);
    try {
      const next = await deleteMainSkill(deleteSkillDirName, true);
      setAppState(next);
      setSelectedTargetId(next.selectedTargetId);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
      await refresh(selectedTargetId);
    } finally {
      setPendingSkillKey(null);
    }
  };

  const handleCancelDeleteMainSkill = () => {
    setDeleteSkillDirName(null);
  };

  const mainSkillsDir = appState?.config.settings.mainSkillsDir ?? null;
  const validSkills = appState?.skills.filter((s) => s.valid) ?? [];
  const invalidSkills = appState?.skills.filter((s) => !s.valid) ?? [];
  const selectedTarget =
    appState?.config.targets.find((t) => t.id === selectedTargetId) ?? null;

  const deleteLinkCount = deleteSkillDirName
    ? appState?.config.installations.filter(
        (i) => i.skillDirName === deleteSkillDirName
      ).length ?? 0
    : 0;

  const deleteMessage = deleteSkillDirName
    ? deleteLinkCount > 0
      ? `Skill '${deleteSkillDirName}' will be permanently deleted. ${deleteLinkCount} recorded target link(s) will be removed first. This action cannot be undone.`
      : `Skill '${deleteSkillDirName}' will be permanently deleted. This action cannot be undone.`
    : '';

  return (
    <div className="app-shell">
      <Sidebar
        mainSkillsDir={mainSkillsDir}
        validSkillCount={validSkills.length}
        invalidSkillCount={invalidSkills.length}
        targets={appState?.config.targets ?? []}
        selectedTargetId={selectedTargetId}
        onSelectTarget={handleSelectTarget}
        onAddTarget={handleAddTarget}
        onEditTarget={handleEditTarget}
        onDeleteTarget={handleDeleteTarget}
        onSetMainSkillsDir={handleSetMainSkillsDir}
        onManageSkills={handleManageSkills}
      />
      <main className="main-panel">
        {loading && <div className="loading-overlay">Loading…</div>}
        {error && (
          <div className="error-banner">
            {error}
            <button
              className="close-button"
              onClick={() => setError(null)}
              aria-label="Dismiss error"
            >
              ×
            </button>
          </div>
        )}
        {mainView === 'main-library' ? (
          <MainLibraryPage
            skills={appState?.skills ?? []}
            validSkillCount={validSkills.length}
            invalidSkillCount={invalidSkills.length}
            onDeleteMainSkill={handleDeleteMainSkill}
          />
        ) : (
          <TargetDetail
            target={selectedTarget}
            skills={appState?.selectedTargetSkills ?? []}
            pendingSkillKey={pendingSkillKey}
            onToggleSkill={handleToggleSkill}
          />
        )}
        <ConfirmDialog
          open={!!deleteSkillDirName}
          title="Confirm Deletion"
          message={deleteMessage}
          confirmLabel="Delete"
          cancelLabel="Cancel"
          danger
          onConfirm={handleConfirmDeleteMainSkill}
          onCancel={handleCancelDeleteMainSkill}
        />
      </main>
    </div>
  );
}

export default App;
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
npm test -- src/test/app.test.tsx
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/test/app.test.tsx
git commit -m "feat: add main view state and wire MainLibraryPage

- Default view is main-library
- Target selection switches to target detail
- Manage Skills button switches back to main library

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 6: 删除 MainLibraryPanel.tsx

**Files:**
- Delete: `src/components/MainLibraryPanel.tsx`

- [ ] **Step 1: Verify no references remain**

Run:
```bash
grep -r "MainLibraryPanel" src/
```

Expected: No output (no references).

- [ ] **Step 2: Delete the file**

```bash
git rm src/components/MainLibraryPanel.tsx
```

- [ ] **Step 3: Run tests to verify nothing breaks**

Run:
```bash
npm test
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: remove obsolete MainLibraryPanel component

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 7: 更新样式

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Add styles for MainLibraryPage and Manage Skills button layout**

在 `src/styles.css` 末尾添加：

```css
/* Main library page (main panel) */
.main-library-page {
  max-width: 900px;
}

.main-library-page h2 {
  margin: 0 0 0.75rem;
  font-size: 1.5rem;
  font-weight: 600;
  color: #111827;
}

.main-library-page h3 {
  margin: 0 0 0.75rem;
  font-size: 1rem;
  font-weight: 600;
  color: #374151;
}

/* Sidebar summary action buttons */
.main-library-panel .secondary-button {
  width: 100%;
  display: block;
}
```

- [ ] **Step 2: Run tests**

Run:
```bash
npm test
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/styles.css
git commit -m "style: add MainLibraryPage and summary button styles

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Task 8: 完整测试套件验证

**Files:**
- All of the above

- [ ] **Step 1: Run full test suite**

Run:
```bash
npm test
```

Expected: All tests PASS.

- [ ] **Step 2: Run TypeScript build check**

Run:
```bash
npm run build
```

Expected: Build succeeds with no TypeScript errors.

- [ ] **Step 3: Commit any final fixes**

If any fixes were needed:

```bash
git add -A
git commit -m "fix: address test/build issues from main library refactor

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Self-Review Checklist

### Spec Coverage

| Spec 要求 | 对应 Task |
|---|---|
| Target 详情只保留安装/卸载 | Task 4 |
| Sidebar Main Library 只显示摘要和入口 | Task 1, Task 3 |
| 独立 Main Library 详情页可删除 skill | Task 2, Task 5 |
| 应用启动默认 Main Library | Task 5 |
| Main Library 显示所有 skill 包括 invalid | Task 2 |
| 复用 ConfirmDialog 和 deleteMainSkill | Task 5 |
| 更新测试 | Task 1, 2, 5, 8 |

### Placeholder Scan

- 无 "TBD"、"TODO"、"implement later"。
- 每个步骤包含实际代码或命令。
- 测试代码完整，无 "write tests for the above"。

### Type Consistency

- `MainView = 'main-library' | 'target'` 在 Task 5 中定义并贯穿使用。
- `onManageSkills` 在 `MainLibrarySummaryProps`、`SidebarProps`、`App.tsx` 中签名一致。
- `onDeleteMainSkill` 在 `MainLibraryPageProps` 和 `App.tsx` 中签名一致。
- `TargetDetailProps` 中已移除 `onDeleteMainSkill`。

### Known Gotchas

- Task 3 单独执行会导致 `App.tsx` TypeScript 编译失败（缺少 `onManageSkills` prop）。建议与 Task 5 连续执行，或在 Task 5 完成后验证 Task 3 的测试。
- Task 4 的测试在 Task 5 完成前会失败，因为默认视图尚未切换到 Main Library / Target detail。实际执行时应在 Task 5 后验证。
- 原有 `baseAppState.selectedTargetId: 'target_1'` 仍然存在，但 App 启动后不再默认渲染 Target 详情；`selectedTargetId` 只在切换到 Target 视图时使用。
