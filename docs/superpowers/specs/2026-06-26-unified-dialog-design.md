# 统一自定义对话框设计

## 目标

将 `src/App.tsx` 中所有原生 `window.prompt` 和 `window.confirm` 替换为与现有 UI 风格一致的自定义对话框组件，提升视觉统一性和用户体验。

## 现状

当前 `App.tsx` 中共有 5 处原生对话框：

- `handleSetMainSkillsDir`：使用 `window.prompt` 输入 Main Library 目录路径。
- `handleAddTarget`：连续使用两个 `window.prompt` 分别输入 Target name 和 Skills directory path。
- `handleEditTarget`：连续使用两个 `window.prompt` 分别输入新的 Target name 和 Skills directory path。
- `handleDeleteTarget`：使用 `window.confirm` 确认删除 Target。
- `handleDeleteTarget` catch 块：使用 `window.confirm` 确认强制删除带有安装记录的 Target。

现有 `ConfirmDialog` 组件已经是自定义对话框，但样式与原生 prompt 不一致。

## 设计决策

| 决策 | 选择 | 理由 |
|---|---|---|
| 替换范围 | 所有 `window.prompt` 和 `window.confirm` | 用户要求统一所有对话框 |
| 基础组件 | 新增通用 `Dialog` 组件 | 抽离通用结构，供 PromptDialog、TargetFormDialog、ConfirmDialog 复用 |
| Prompt 对话框 | `PromptDialog`（单输入） | 设置目录只需要一个输入框 |
| Target 表单对话框 | `TargetFormDialog`（双输入） | Add/Edit Target 需要 name 和 path 两个字段 |
| Confirm 对话框 | 重构现有 `ConfirmDialog` 使用 `Dialog` | 保持 API 不变，统一视觉风格 |
| 样式策略 | 将现有 `.confirm-dialog-*` 升级为通用 `.dialog-*` | 去掉 confirm 专属语义，使其可复用 |
| 表单验证 | 空值禁用提交按钮 | 最小验证，避免无效提交 |

## 架构

```
Dialog (基础容器)
├── PromptDialog    (单输入，用于设置 Main Directory)
├── TargetFormDialog (双输入，用于 Add/Edit Target)
└── ConfirmDialog    (确认信息 + 确认/取消按钮)
```

`App.tsx` 通过状态控制各对话框的打开与内容，回调函数接收用户输入后调用原有 command API。

## 组件设计

### `Dialog` 基础组件

**Props:**

```ts
export interface DialogProps {
  open: boolean;
  title: string;
  children: React.ReactNode;
  actions: React.ReactNode;
  onClose?: () => void;
}
```

**职责：**
- 渲染 overlay、card、header、body、actions。
- 如果传了 `onClose`，按 Escape 键触发关闭，点击 overlay 触发关闭。
- 不处理任何业务逻辑。

### `PromptDialog` 组件

**Props:**

```ts
export interface PromptDialogProps {
  open: boolean;
  title: string;
  label: string;
  defaultValue?: string;
  confirmLabel?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}
```

**职责：**
- 基于 `Dialog`。
- 渲染一个 label + input。
- 确认时把当前 input 值传给 `onConfirm`。
- 取消时调用 `onCancel`。
- 用于设置 Main Library 目录路径。

### `TargetFormDialog` 组件

**Props:**

```ts
export interface TargetFormDialogProps {
  open: boolean;
  title: string;
  initialName?: string;
  initialSkillsDir?: string;
  confirmLabel?: string;
  onConfirm: (name: string, skillsDir: string) => void;
  onCancel: () => void;
}
```

**职责：**
- 基于 `Dialog`。
- 渲染两个输入框：Target name、Skills directory path。
- 两个字段均非空时才允许点击确认。
- Add Target 时初始值为空；Edit Target 时预填充当前值。

### `ConfirmDialog` 组件（重构）

保持现有 Props 不变：

```ts
export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}
```

**变更：**
- 内部实现改为使用 `Dialog` 组件。
- 继续支持 `danger` prop，用于确认按钮显示危险样式。

## App.tsx 状态调整

新增状态：

```ts
const [targetFormOpen, setTargetFormOpen] = useState(false);
const [targetFormTarget, setTargetFormTarget] = useState<Target | null>(null);
const [promptDialogOpen, setPromptDialogOpen] = useState(false);
const [promptDialogDefaultValue, setPromptDialogDefaultValue] = useState('');
```

事件处理调整：

- `handleSetMainSkillsDir`：
  - 设置 `promptDialogDefaultValue` 为当前 `mainSkillsDir`。
  - 打开 `PromptDialog`。
  - 确认回调中调用 `setMainSkillsDir(path)`。

- `handleAddTarget`：
  - 设置 `targetFormTarget` 为 `null`。
  - 打开 `TargetFormDialog`。
  - 确认回调中调用 `addTarget(name, skillsDir)`。

- `handleEditTarget(target)`：
  - 设置 `targetFormTarget` 为当前 target。
  - 打开 `TargetFormDialog`。
  - 确认回调中调用 `updateTarget(target.id, name, skillsDir)`。

- `handleDeleteTarget`：
  - 保持使用 `ConfirmDialog`，无需新增对话框状态（已有 `deleteSkillDirName` 控制的是 skill 删除确认）。
  - 新增 `deleteTargetConfirmOpen` 和 `deleteTargetData` 状态来控制 Target 删除确认对话框。

## 数据流

1. 用户点击 Add Target / Edit Target / Change Directory。
2. `App.tsx` 设置对应对话框状态（打开 + 初始值）。
3. 用户在对话框中输入并点击确认。
4. 对话框回调把值传给 `App.tsx` 的 handler。
5. `App.tsx` 调用原有 command API（`addTarget`、`updateTarget`、`setMainSkillsDir`）。
6. command 成功后关闭对话框、刷新 `appState`、显示错误或成功状态。

## 错误处理

- command 调用失败时，继续通过现有 `error-banner` 显示错误信息。
- 对话框在错误发生时不自动关闭，方便用户修改后重试。
- 空值验证在对话框内部完成，未填写时禁用确认按钮。

## 测试

### 新增测试

- `src/test/Dialog.test.tsx`：
  - 渲染 Dialog。
  - 按 Escape 触发 `onClose`。
  - 点击 overlay 触发 `onClose`。
  - 渲染 title、children、actions。

- `src/test/PromptDialog.test.tsx`：
  - 渲染 label 和 default value。
  - 输入后点击确认，回调收到输入值。
  - 点击取消，回调未被调用。

- `src/test/TargetFormDialog.test.tsx`：
  - 渲染两个输入框。
  - 空值时确认按钮被禁用。
  - 填写后点击确认，回调收到 name 和 skillsDir。
  - Edit 模式预填充初始值。

### 更新测试

- `src/test/ConfirmDialog.test.tsx`：
  - 确认重构后行为不变（打开、确认、取消、danger 样式）。

- `src/test/app.test.tsx`：
  - 移除对 `window.prompt` 的依赖。
  - Add Target 测试改为打开 `TargetFormDialog` 并填写提交。
  - Edit Target 测试改为打开 `TargetFormDialog` 并修改提交。
  - Set Main Directory 测试改为打开 `PromptDialog` 并输入路径。

## 样式

将 `src/styles.css` 中的 `.confirm-dialog-*` 类名改为通用 `.dialog-*`，或同时保留两套类名作为别名：

```css
.dialog-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
  z-index: 50;
}

.dialog {
  background: #ffffff;
  border-radius: 0.5rem;
  box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05);
  max-width: 28rem;
  width: 90%;
  padding: 1.25rem;
}

.dialog-header {
  font-size: 1.125rem;
  font-weight: 600;
  color: #111827;
  margin-bottom: 0.75rem;
}

.dialog-body {
  font-size: 0.9375rem;
  color: #374151;
  line-height: 1.5;
  margin-bottom: 1.25rem;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.dialog-form-field {
  margin-bottom: 0.75rem;
}

.dialog-form-field label {
  display: block;
  font-size: 0.875rem;
  color: #374151;
  margin-bottom: 0.25rem;
}

.dialog-form-field input {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.375rem;
  font-size: 0.9375rem;
}
```

## 文件改动清单

- 新增 `src/components/Dialog.tsx`
- 新增 `src/components/PromptDialog.tsx`
- 新增 `src/components/TargetFormDialog.tsx`
- 修改 `src/components/ConfirmDialog.tsx`（内部使用 Dialog）
- 修改 `src/App.tsx`（替换 prompt/confirm，管理对话框状态）
- 修改 `src/styles.css`（添加/调整通用对话框样式）
- 新增 `src/test/Dialog.test.tsx`
- 新增 `src/test/PromptDialog.test.tsx`
- 新增 `src/test/TargetFormDialog.test.tsx`
- 修改 `src/test/ConfirmDialog.test.tsx`
- 修改 `src/test/app.test.tsx`

## 未解决问题

无。
