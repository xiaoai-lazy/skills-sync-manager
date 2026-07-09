# P2-2 设计：CSS 按域拆分

> 日期：2026-07-09  
> 状态：已批准（brainstorming）  
> 关联：[架构审查](./2026-07-08-architecture-review.md)  
> 方案选择：**按域机械切开 + `@import` 入口**（零视觉改动）

---

## 1. 目标

将 `src/styles.css`（约 2980 行）拆成 `src/styles/` 下多个域样式表，入口 `src/styles.css` 仅 `@import` 聚合。`main.tsx` 的 `import './styles.css'` **不改**。

## 2. 策略（已确认）

| 议题 | 选择 |
|------|------|
| 拆分方式 | 普通全局 CSS，非 CSS Modules |
| 入口 | 保留 `src/styles.css` 为 `@import` 聚合 |
| 粒度 | 约 9～10 个域文件（合并小段） |
| 规则内容 | **连续行号切分**，不改选择器、不整理去重、不重排级联 |

## 3. 文件划分（实施结果）

为保持与原文完全相同的级联顺序，切分必须连续；因此 `shell.css` 含 sidebar 导航与 main panel，`overlays.css` 承接原文中位于 hub 之后的 switch / PAT modal。

| 文件 | 原行号（约） | 内容 |
|------|--------------|------|
| `styles/tokens.css` | 1–123 | design tokens |
| `styles/scrollbars.css` | 124–283 | scrollbars |
| `styles/shell.css` | 284–555 | frameless、app-shell、sidebar 导航、main panel |
| `styles/sidebar.css` | 556–695 | target list、project tree |
| `styles/target.css` | 696–851 | target detail |
| `styles/controls.css` | 852–959 | status badges、buttons |
| `styles/feedback.css` | 960–1039 | error banner、migration toast、loading |
| `styles/dialogs.css` | 1040–1288 | confirm / generic dialog、empty state |
| `styles/hub.css` | 1289–2675 | Skill Hub + main library page |
| `styles/overlays.css` | 2676–end | iOS switch、PAT/keys modal |
| `styles.css` | — | 仅按上表顺序 `@import` |

`@import` 顺序 = 上表顺序 = 原文件级联顺序。

## 4. 明确不做

- CSS Modules / 改类名 / 视觉改版  
- 按组件目录就近放置  
- `@layer`、选择器去重整理  
- 为「域名更纯」而重排非连续段落（会破坏级联）  
- 拆 `hub.css` 内部（可留后续）

## 5. 验收

- 切分后拼接内容与原 `styles.css` 字节级一致（实施脚本断言）  
- `npm run build` 通过  
- 现有 Vitest 全绿  
- `styles.css` 仅保留 `@import` 入口  

## 6. 实施

脚本按行号切分 → 写各域文件 → 替换入口 → build + test → commit。
