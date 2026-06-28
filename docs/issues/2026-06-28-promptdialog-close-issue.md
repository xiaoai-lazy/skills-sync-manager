# PromptDialog 关闭行为问题排查记录

日期：2026-06-28

## 目标

让 `PromptDialog` 只能通过 **Cancel** 和 **Save** 两个按钮关闭：

- 点击对话框外部灰色 overlay 不应关闭。
- 按 ESC 键不应关闭。

## 现象

用户反馈：在运行的 Tauri 应用中，打开 `PromptDialog`（如点击 sidebar 的 **Change Directory**）后：

1. 点击窗口外部（overlay 区域）仍然会关闭对话框。
2. 按 ESC 键仍然会关闭对话框。

## 已做尝试

### 1. 移除 PromptDialog 的 `onClose` 传递

修改 `src/components/PromptDialog.tsx`，不再向 `Dialog` 传递 `onClose={onCancel}`。

预期：`Dialog` 没有 `onClose` prop，就不会响应 Escape 和 overlay 点击。

结果：单元测试通过，但用户实际体验仍有问题。

### 2. 强化 Dialog 内部判断

修改 `src/components/Dialog.tsx`：

- `useEffect` 中仅在 `onClose` 存在时才注册 Escape 监听。
- overlay 的 `onClick` 仅在 `onClose` 存在时才附加。

```tsx
useEffect(() => {
  if (!open || !onClose) return;
  // ...
}, [open, onClose]);

// overlay
onClick={onClose ? () => onClose() : undefined}
```

预期：没有 `onClose` 时， Escape 和 overlay 点击完全无效。

结果：单元测试通过，但用户实际体验仍有问题。

### 3. 添加显式关闭控制 prop

在 `Dialog` 上新增：

```ts
closeOnEscape?: boolean;        // default: true
closeOnOverlayClick?: boolean;  // default: true
```

并在 `PromptDialog` 中显式禁用：

```tsx
<Dialog
  open={open}
  title={title}
  closeOnEscape={false}
  closeOnOverlayClick={false}
  // ...
/>
```

预期：即使 `onClose` 被某些机制传入，`PromptDialog` 也明确禁止 ESC 和 overlay 关闭。

结果：单元测试通过，等待用户重新验证。

### 4. 多次重启 Tauri 应用

- 杀掉 `skills-sync-manager.exe` 进程。
- 清理 `node_modules/.vite` 缓存。
- 释放 1420 端口。
- 重新执行 `npm run tauri:dev`。

预期：冷启动确保加载最新前端代码。

结果：用户反馈问题依旧。

### 5. 验证 Vite 实际 served 的源码

通过 `curl http://localhost:1420/src/components/Dialog.tsx` 检查运行时源码，确认：

- `PromptDialog` 没有传递 `onClose`。
- `Dialog` 的 Escape 处理和 overlay click 都依赖 `onClose`。
- 最新添加的 `closeOnEscape={false}` 和 `closeOnOverlayClick={false}` 已生效。

结果：运行时源码与本地源码一致，逻辑上不应关闭。

### 6. 单元测试覆盖

新增/更新测试：

- `Dialog.test.tsx`：验证无 `onClose` 时 ESC 不关闭。
- `PromptDialog.test.tsx`：验证 ESC 和 overlay 点击不关闭，仅 Cancel/Save 按钮关闭。

结果：所有 54 个测试通过。

### 7. 尝试 Playwright 自动化验证

想通过 Playwright 启动 Chromium 并模拟点击/ESC 来复现问题。但本地项目未安装 `playwright` 包，`npx playwright` 运行时模块解析失败，暂未成功运行端到端验证。

## 当前代码状态

- `src/components/Dialog.tsx`：已增加 `closeOnEscape` / `closeOnOverlayClick` 显式控制。
- `src/components/PromptDialog.tsx`：已显式设置 `closeOnEscape={false}`、`closeOnOverlayClick={false}`。
- `src/test/PromptDialog.test.tsx`：已覆盖 ESC 和 overlay 不关闭的场景。
- 测试：`npm test` 54/54 通过。
- 应用：已重新冷启动，窗口 **"Skills Sync Manager"** 正在运行。

## 未解问题

在单元测试和源码逻辑均正确的情况下，用户在实际 Tauri 窗口中仍能复现 ESC / overlay 点击关闭 `PromptDialog`。

## 可能的下一步

1. **确认用户具体操作**：请用户再次明确点击的是对话框外部灰色区域（而非 Cancel 按钮），以及按 ESC 时输入框是否聚焦。
2. **浏览器端验证**：在 Tauri 窗口中按 F12 打开 DevTools，检查 Elements 中 `.dialog-overlay` 是否确实没有绑定 click listener。
3. **进一步隔离**：创建一个最小化的 Tauri 对话框页面，排除 App.tsx 中其他状态或事件的影响。
4. **安装 Playwright**：将 `playwright` 作为 devDependency 安装，编写端到端测试复现真实窗口行为。
5. **检查 Tauri WebView 默认行为**：确认 WebView2 是否对 `role="dialog"` 或 `aria-modal="true"` 有默认的 ESC/overlay 关闭行为。

## 相关文件

- `src/components/Dialog.tsx`
- `src/components/PromptDialog.tsx`
- `src/test/PromptDialog.test.tsx`
- `src/test/Dialog.test.tsx`
