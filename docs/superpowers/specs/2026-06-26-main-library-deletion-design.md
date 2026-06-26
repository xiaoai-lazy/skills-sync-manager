# Main Library 独立删除页面设计

## 目标

- Target 详情页只保留安装/卸载操作，不再提供删除 skill 的入口。
- Sidebar 中的 Main Library 区域只显示摘要信息（目录路径、skill 总数、valid/invalid 数量）和「Manage Skills」入口。
- 新增独立的 Main Library 详情页，用于查看所有已管理的 skill，并在此页面提供删除功能。
- 应用启动后默认打开 Main Library 详情页。

## 现状

当前代码中：

- `MainLibraryPanel.tsx` 已经实现了完整的 skill 列表和删除按钮，但 `Sidebar.tsx` 只使用了它的摘要 props，没有传入 `skills` 和 `onDeleteMainSkill`。
- `App.tsx` 的主面板固定渲染 `TargetDetail`。
- `TargetDetail.tsx` 声明了 `onDeleteMainSkill` prop，但实际上没有使用；`SkillRow` 只提供安装/卸载 checkbox。
- 删除 skill 的确认对话框和关联 installation 清理逻辑已经在 `App.tsx` 和 `ConfirmDialog.tsx` 中实现。

因此，本设计主要是重新组织现有组件和状态，把删除功能从 Target 上下文彻底移到 Main Library 上下文。

## 设计决策

| 决策 | 选择 | 理由 |
|---|---|---|
| Main Library 入口位置 | Sidebar 摘要 + 主面板详情页 | 用户明确希望在独立页面管理所有 skill |
| 主视图切换方式 | 单视图互斥 | 状态简单，同一时间只看一个视图 |
| 实现方式 | 组件拆分 | 职责清晰，Sidebar 和主面板解耦 |
| 默认视图 | Main Library 详情页 | 用户要求默认打开状态 B |
| Invalid skill 显示 | Main Library 详情页显示所有 skill，包括 invalid | 管理页面需要完整视图，invalid skill 仍可删除 |
| Target 详情删除 | 移除 | 用户要求 Target 下只有安装/卸载 |

## 架构

在 `App.tsx` 中新增一个顶层视图状态：

```ts
type MainView = 'main-library' | 'target';
```

- 默认值：`main-library`。
- 点击 Sidebar 中的「Manage Skills」→ 设置 `mainView = 'main-library'`。
- 点击 Target 列表中的某个 target → 设置 `mainView = 'target'` 并更新 `selectedTargetId`。

主面板根据 `mainView` 渲染：

- `'main-library'` → `MainLibraryPage`
- `'target'` → `TargetDetail`

## 组件变更

### 新增/拆分

- `MainLibrarySummary.tsx`：Sidebar 中的摘要卡片。
  - Props：`mainSkillsDir`、`validSkillCount`、`invalidSkillCount`、`onSetMainSkillsDir`、`onManageSkills`。
- `MainLibraryPage.tsx`：主面板中的详情页。
  - Props：`skills`、`validSkillCount`、`invalidSkillCount`、`onDeleteMainSkill`。

### 修改

- `MainLibraryPanel.tsx`：
  - 拆分为上述两个组件后直接删除，避免与新的 `MainLibrarySummary` / `MainLibraryPage` 混淆。
- `Sidebar.tsx`：
  - 引入 `MainLibrarySummary` 替代 `MainLibraryPanel`。
  - 新增 `onManageSkills` prop 并透传给 `MainLibrarySummary`。
- `TargetDetail.tsx`：
  - 移除 `onDeleteMainSkill` prop。
  - 内部只渲染 `SkillRow`，只保留安装/卸载 checkbox。
- `App.tsx`：
  - 新增 `mainView` 状态。
  - 主面板条件渲染 `MainLibraryPage` 或 `TargetDetail`。
  - 删除相关状态和回调保持不变（`deleteSkillDirName`、`handleDeleteMainSkill`、`handleConfirmDeleteMainSkill` 等）。
  - `handleSelectTarget` 中同时设置 `mainView = 'target'`。

### 不变

- `SkillRow.tsx`：只负责展示 skill 信息和安装/卸载切换，不参与删除。
- `ConfirmDialog.tsx`：继续用于删除确认。
- `commands.ts` / Rust 后端：`deleteMainSkill` command 不变。

## 数据流

1. `MainLibraryPage` 接收 `skills` 列表和 `onDeleteMainSkill(skillDirName)` 回调。
2. 用户点击删除 → `App.tsx` 的 `handleDeleteMainSkill` 设置 `deleteSkillDirName` → 打开 `ConfirmDialog`。
3. 确认后调用 `deleteMainSkill(skillDirName, true)`。
4. 成功后更新 `appState`；`mainView` 保持 `'main-library'`。
5. 如果删除失败，显示 `error-banner` 并刷新状态。

## 错误处理

- 复用现有 `error-banner` 显示操作失败信息。
- `ConfirmDialog` 继续显示关联 installation 数量警告：
  - 若该 skill 已安装到 N 个 target，提示将先移除这些链接。
  - 若未安装，提示永久删除且不可恢复。

## 测试

更新 `src/test/app.test.tsx`，覆盖：

- 应用启动后默认渲染 `MainLibraryPage`，不渲染 `TargetDetail`。
- `MainLibraryPage` 中列出所有 skill（包括 invalid），每个 skill 项都有删除按钮。
- 点击「Manage Skills」保持在 Main Library 视图。
- 点击 Target 列表切换到 Target 详情视图。
- `TargetDetail` 中的 `SkillRow` 只有 checkbox，没有删除按钮。
- 在 Main Library 视图中点击删除会弹出确认对话框，确认后调用 `deleteMainSkill` command。
- 删除失败时显示错误提示。

## 未解决问题

无。

## 附录：文件改动清单

- 新增 `src/components/MainLibrarySummary.tsx`
- 新增 `src/components/MainLibraryPage.tsx`
- 删除 `src/components/MainLibraryPanel.tsx`
- 修改 `src/components/Sidebar.tsx`
- 修改 `src/components/TargetDetail.tsx`
- 修改 `src/App.tsx`
- 修改 `src/test/app.test.tsx`
- 修改 `src/App.css`（如需要调整新组件样式）
