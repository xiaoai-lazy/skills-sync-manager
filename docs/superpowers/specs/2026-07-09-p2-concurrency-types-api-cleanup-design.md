# P2 设计（本轮）：并发可见性、类型对齐、API 小清理

> 日期：2026-07-09  
> 状态：已批准（brainstorming）  
> 关联：[架构审查](./2026-07-08-architecture-review.md)、[P1 设计](./2026-07-09-p1-design.md)  
> 方案选择：**三阶段小 PR**（P2-3 → P2-1 → P2-4）

---

## 1. 范围与目标

### 1.1 本轮做什么

| 编号 | 目标 | 用户已选策略 |
|------|------|----------------|
| **P2-3** | 发现/更新启动竞态与忙时可见性 | 启动串行；启动遇 InProgress 静默；手动刷新才提示 |
| **P2-1** | 前后端字段对齐 + 防漂移 | 补齐缺口 + 简单键集合对比检查 |
| **P2-4** | API/模型小清理 | 去掉 `updateTarget` 无用 `skillsDir`；标明 `LinkStrategy` 仅 `auto` |

### 1.2 交付节奏

1. P2-3 并发与可见性  
2. P2-1 字段对齐 + 检查  
3. P2-4 `update_target` / `LinkStrategy` 清理  

可独立提交/审查；顺序固定。

### 1.3 明确不做（本轮）

- 后端共享写锁 / 单队列串行化所有 discover·updates（P2-3 方案 B/C）
- 类型 codegen / ts-rs / 单一生成真相源（P2-1 方案 C）
- `DiscoverableSkill` 与 `SkillRecord` 字段大去重
- 允许编辑目标时修改 `skillsDir`（产品行为保持只读）
- P2-2 CSS 拆分、P2-5 SourceTree 解耦、P2-6 迁移 ADR
- P2-7（文档已与嵌套扫描一致，移出清单）

### 1.4 成功标准

1. 启动不再并行触发会互相 `save` 的 discover 与 checkUpdates  
2. 启动路径对 `discoverInProgress` / `updatesInProgress` 不刷错误条；手动刷新有「进行中」反馈  
3. TS `AppConfig` 含 `gitlabCredentialHosts`；键对比检查能拦住回归  
4. `updateTarget` / `update_target` 不再接收无用的 `skillsDir`；`LinkStrategy` 文档标明仅 `auto`

---

## 2. 阶段 1 — P2-3 并发与可见性

### 2.1 问题

- 后端：`DISCOVER_IN_PROGRESS` / `UPDATES_IN_PROGRESS` 两把独立锁；discover 与 updates **可并行**，各自 `load → 改缓存 → save`，存在写覆盖风险  
- 前端启动：`useAppBootstrap` 中 `Promise.all([runBackgroundDiscover(), runBackgroundCheckUpdates()])` 放大该竞态  
- 忙时：本地 in-flight 直接 `return` 无提示；后端 InProgress 若当普通失败，文案像故障

### 2.2 策略（已确认：方案 A + 可见性 C）

**后端：** 保留现有 AtomicBool 与错误码；本轮不加共享写锁/队列。

**启动（`useAppBootstrap`）：**

```
await runBackgroundDiscover()
await runBackgroundCheckUpdates()
```

串行顺序固定：先 discover，再 checkUpdates。

**可见性：**

| 路径 | InProgress / 本地已 in-flight | 其它错误 |
|------|-------------------------------|----------|
| 启动后台 | **静默**（不 `setError`） | 仍 `setError` |
| 用户手动刷新（Hub / 显式按钮） | 轻提示「正在刷新，请稍候」（优先 `onToast`；无则非故障语气的提示） | 现有错误处理 |

错误码（已有）：`discoverInProgress`、`updatesInProgress`。

### 2.3 主要改动文件

- `src/hooks/useAppBootstrap.ts` — 串行启动；过滤 InProgress  
- `src/hooks/useSkillHub.ts` — 启动用 runner 静默 InProgress；必要时导出错误分类 helper  
- `src/components/skill-hub/SkillHubPage.tsx` — 手动刷新遇忙时提示（若尚未完善）  
- 可选：`src/utils/errorMessage.ts` 或小工具识别 InProgress code  
- 测试：`src/test/app.test.tsx` 等；必要时补 InProgress 静默用例

### 2.4 验收

- 启动路径代码审查：无 `Promise.all` 并行 discover+updates  
- 手动连点刷新有反馈；启动不因 InProgress 弹错误条  
- `npm test`（排除 worktree 污染时用 vitest exclude）全绿  

### 2.5 明确不做

- 后端串行化两类任务或合并为单锁  
- 改 runtime-cache / config 持久化策略  

---

## 3. 阶段 2 — P2-1 字段对齐 + 防漂移

### 3.1 问题

- `models.rs` `AppConfig.gitlab_credential_hosts` 存在；`types.ts` `AppConfig` 无对应字段  
- 前后端手写同步，易再漏

### 3.2 策略（已确认：方案 B）

1. **对齐：** `AppConfig` 增加 `gitlabCredentialHosts?: string[]`；再扫一轮共享 DTO，只补明显缺口（Rust↔TS 键名 snake/camel）  
2. **检查：** Vitest（或脚本）对比 `AppConfig` 字段集合；显式 ignore 列表；故意删字段应失败  
3. **不做** codegen

### 3.3 主要改动文件

- `src/model/types.ts`  
- `src/test/appConfigFieldsAlign.test.ts`（或 `scripts/` + 测试调用）  
- 若需解析 Rust：只读 `models.rs` 中 `struct AppConfig` 块，保持检查简单  

### 3.4 验收

- 类型齐全；对比测试绿；CI 经 `npm test` 覆盖  

### 3.5 明确不做

- 全量结构体双向生成  
- 为 credential hosts 新做 UI  

---

## 4. 阶段 3 — P2-4 API 小清理

### 4.1 问题

- `update_target` 接收 `skills_dir` 但实现忽略；前端 `updateTarget(id, name, skillsDir)` 仍传路径  
- `LinkStrategy` 仅 `Auto`，缺少「仅预留」说明  

### 4.2 策略（已确认：方案 B）

1. 后端去掉 `skills_dir` 参数；请求类型收窄为改 name  
2. 前端 `updateTarget(targetId, name)`；编辑对话框仍只读展示路径，确认时不传 path  
3. README/注释：`LinkStrategy` 当前仅 `auto`  
4. **不做** DiscoverableSkill 字段去重；**不**开放改 skillsDir  

### 4.3 主要改动文件

- `src-tauri/src/commands/mod.rs`、`target_registry.rs`（若 request 形状变化）  
- `src/api/commands.ts`、`useTargetActions.ts`、`TargetFormDialog` / `App.tsx` 调用处  
- `src/test/app.test.tsx` 等断言  
- `README.md` / `README.zh.md` 一句  

### 4.4 验收

- 前后端签名一致；编辑目标名称行为不变；相关测试绿  

---

## 5. 测试与回归

每阶段结束：

```bash
npx vitest run src/test --exclude ".worktrees/**"
cargo test --manifest-path src-tauri/Cargo.toml   # 阶段 3 必跑；阶段 1–2 若未改 Rust 可跳过
```

---

## 6. 架构审查 P2 清单状态（本轮后）

| 编号 | 本轮 | 备注 |
|------|------|------|
| P2-3 | 做（串行+可见性） | 不加写锁/队列 |
| P2-1 | 做（对齐+检查） | 不做 codegen |
| P2-4 | 做（小清理） | 不做 DiscoverableSkill 大去重 |
| P2-2 | 不做 | CSS |
| P2-5 | 不做 | SourceTree |
| P2-6 | 不做 | ADR |
| P2-7 | 关闭 | 文档已一致 |

---

## 7. 实施交接

批准本设计后：用 `writing-plans` 产出 `docs/superpowers/plans/2026-07-09-p2-concurrency-types-api-cleanup.md`，再按三阶段实现。
