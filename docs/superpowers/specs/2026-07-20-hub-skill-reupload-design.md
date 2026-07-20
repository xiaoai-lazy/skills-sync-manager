# Skill Hub 主库重传 Design

## Goal

让用户在主库中修改来自 Skill Hub 的 skill 后，能从 Skill 中心「已安装」卡片一键重新上传到原 Hub 目标；同时让「更新」（Hub → 本地）在存在本地修改时先确认，避免误覆盖。

## Current Behavior

- 本地 → Hub：已有 `UploadToHubDialog` + `upload_skill_to_hub`，仅从主库 `storageKey` 解析路径并 zip 上传；上传后刷新 discover 缓存，**不**写回 `skill_records.content_hash`。
- Hub → 本地：卡片「更新」调用 `update_skill`，无二次确认。
- 「有更新」= 当前主库 hash ≠ 远程 hash（`check_hub_record`）。
- 应用内无 skill 编辑器；编辑依赖外部工具改主库文件。
- 目标目录通过 junction/symlink 指向主库；目标目录新建/分叉与主库对账不在本功能范围。

## Desired Behavior

1. 仅 **Hub 来源**（`source === skillhub`，且具备 `hubEndpointId` / `hubSkillGroup` / `hubSkillId`）的已安装 skill 参与本功能。
2. **已修改**判定：主库目录存在，且 `currentLocalHash ≠ record.contentHash`（算法与写入 `contentHash` 时一致）。
3. 卡片在「已修改」时显示徽章 **「已修改」**；**仅此时**显示「重新上传」。
4. 点击「重新上传」→ 确认「将覆盖远程」→ 调用现有 `uploadSkillToHub(endpointId, group, storageKey)`（静默目标 = 原 endpoint / group / skill id，不可改）。
5. 重传成功后：将对应 `skill_records.content_hash` 更新为当前主库 hash；清除「已修改」；刷新 discover 与 pending updates；toast 成功。
6. 点击「更新」：
   - 无本地修改：直接拉取（不弹确认）。
   - 有本地修改：确认「将覆盖本地」后再拉取。
7. 拉取成功后沿用现有逻辑刷新 `contentHash`；清除「已修改」与「有更新」（若已对齐）。
8. endpoint 禁用/缺失、目录不存在、上传失败：toast 错误，不改 `contentHash`。

## Out of Scope

- 应用内编辑器
- 目标目录 ↔ 主库同步 / 从 Agent 目录回传
- GitHub / GitLab / 本地来源的一键重传（继续走「上传到 Hub」对话框）
- 重传时改分组或 endpoint
- 批量重传、三路合并

## UI

- 页面：Skill 中心（`SkillHubPage`）「已安装」页签。
- 组件：`SkillCard` 增加 `localDirty`、`onReupload`；徽章「已修改」（warning 风格，对齐原型）。
- 操作区（Hub）：`[更新?][重新上传?][删除]` ——「更新」仍由 `hasUpdate` 控制；「重新上传」仅由 `localDirty` 控制。
- 确认：复用 `ConfirmDialog`（或等价），文案需标明覆盖方向与目标 meta（Hub / 分组 / skill id）。
- 原型：`docs/prototypes/v0.8-hub-reupload-prototype.html`（本地参考；`docs/` 目录被 gitignore，以本 spec 为准）。

## Data Flow

### 已修改（localDirty）

```
scan / hub state refresh
  → for each hub SkillRecord with local dir:
       currentHash = hash(main_library_path)
       localDirty = currentHash != record.contentHash
  → expose to UI (prefer bundling into scan/hub state; avoid per-card IPC)
```

Hash 规则与现有 Hub 更新检测一致：优先与 `contentHash` / 远程 hash 所用算法对齐（`compute_skill_md_hash_prefix` vs `compute_dir_hash`，见 `skill_updates.rs`）。

### 重新上传

```
click 重新上传 (only if localDirty)
  → ConfirmDialog: 覆盖远程
  → upload_skill_to_hub(hubEndpointId, group, storageKey)
  → on success:
       record.content_hash = current main-library hash
       persist config
       refresh discover + check_skill_updates + hub UI state
       toast 成功
  → on failure: toast 错误; content_hash unchanged
```

### 更新（拉取）

```
click 更新 (hasUpdate)
  → if localDirty: ConfirmDialog 覆盖本地 → else skip confirm
  → update_skill (existing)
  → on success: existing content_hash refresh; clear dirty/update badges as applicable
```

## Architecture / Components

| Area | Change |
|------|--------|
| Rust `skill_hub_upload` / command | After successful upload, update `skill_records[storage_key].content_hash` and persist |
| Rust scan / hub local state | Compute and return `localDirty` (or current hash) for installed hub skills |
| TS types | Extend `SkillView` / hub state as needed with `localDirty` |
| `SkillCard` | Badges + `onReupload`; gate button on `localDirty` |
| `SkillHubPage` | Wire confirm dialogs for reupload + conditional update confirm; call APIs; toast |
| CSS (`hub.css`) | `.badge-dirty` / 「已修改」样式 |

Keep reusing `uploadSkillToHub` / `upload_skill_to_hub` — no new Hub HTTP API.

## Error Handling

| Case | Behavior |
|------|----------|
| Hub endpoint disabled / missing | Disable or fail reupload with clear toast |
| Main library path missing | Fail upload; toast |
| Network / Hub API error | Toast; no content_hash write |
| User cancels confirm | No request |

## Testing

- Hub + dirty：显示「已修改」与「重新上传」；非 dirty / 非 Hub：不显示重传。
- 重传：确认后 invoke `uploadSkillToHub`；成功后 contentHash 刷新、徽章消失。
- 更新：无 dirty 不弹窗；有 dirty 弹「覆盖本地」。
- 取消确认不发起请求；上传失败不改 contentHash。
- 覆盖现有 `SkillCard` / `SkillHubPage` / upload API 测试并补充上述分支。

## Decisions Log

| Decision | Choice |
|----------|--------|
| 编辑方式 | 外部编辑主库；无应用内编辑器 |
| 谁可重传 | 仅已安装 Hub 来源 |
| 重传目标 | 原 endpoint / group / skill id |
| 重传入口 | 已安装卡片；仅「已修改」时显示 |
| 重传确认 | 始终确认覆盖远程 |
| 更新确认 | 仅 localDirty 时确认覆盖本地 |
| 本地改动检测 | currentHash ≠ contentHash |
| 目标目录 | 本版不做同步 |
