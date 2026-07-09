# P1：CI + runtime-cache + Command 精简 + App hooks — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落地架构审查 P1：PR 级 CI、发现/更新缓存迁出 `config.json`、mutation 减少主库全量扫描、用自定义 hooks 拆分 `App.tsx`。

**Architecture:** 四阶段可独立 PR。① `ci.yml` 在 `master`/`feat/v0.6` 跑 `npm test` + `cargo test`。② 新增 `runtime_cache.rs` 读写 `{app_data}/runtime-cache.json`；load 时从 `config` 迁移缓存字段；discover/updates 只写 runtime-cache。③ `build_app_state` 分 Full/Light：未改主库的 mutation 复用 skills 快照、不调用 `list_skills`；前端 `mergeAppState` 兜底。④ 抽出 `hooks/*`，删除 hub↔app 对 `skillRecords` 的双向同步。

**Tech Stack:** Tauri 2, Rust 2021, React 18, Vitest, GitHub Actions (`windows-latest`).

**Design spec:** [docs/superpowers/specs/2026-07-09-p1-design.md](../specs/2026-07-09-p1-design.md)

---

## File Map

| 文件 | 阶段 | 操作 | 职责 |
|------|------|------|------|
| `.github/workflows/ci.yml` | 1 | 创建 | PR/push 测试门禁 |
| `src-tauri/src/runtime_cache.rs` | 2 | 创建 | runtime-cache 读写、迁移、原子写 |
| `src-tauri/src/lib.rs` | 2 | 修改 | 注册 `runtime_cache` |
| `src-tauri/src/config_store.rs` | 2 | 修改 | load 时迁移缓存出 config |
| `src-tauri/src/commands/mod.rs` | 2–3 | 修改 | 组装 AppState 时注入缓存；Full/Light |
| `src-tauri/src/commands/skill_hub.rs` | 2 | 修改 | discover/updates 写 runtime-cache |
| `src-tauri/src/skill_discover.rs` | 2 | 修改 | 缓存写入改走 runtime API（或由 command 层写） |
| `src-tauri/src/skill_updates.rs` | 2 | 修改 | 同上 |
| `README.md` / `README.zh.md` | 2 | 修改 | 说明 runtime-cache |
| `src/utils/mergeAppState.ts` | 3 | 创建 | 合并 Light/Full 返回 |
| `src/test/mergeAppState.test.ts` | 3 | 创建 | merge 单测 |
| `src/App.tsx` | 3–4 | 修改 | 先接 merge；再改为 hooks 组装 |
| `src/hooks/useAppBootstrap.ts` | 4 | 创建 | 启动加载与后台任务 |
| `src/hooks/useAppState.ts` | 4 | 创建 | appState + 导航选中态 |
| `src/hooks/useSkillHub.ts` | 4 | 创建 | discover/updates；单向派生 |
| `src/hooks/useTargetActions.ts` | 4 | 创建 | 目标/安装相关 |
| `src/hooks/useProjectActions.ts` | 4 | 创建 | 项目相关（可与 target 合并） |
| `src/hooks/useAppDialogs.ts` | 4 | 创建 | dialog 状态 |

---

## Phase 1：CI（P1-4）

### Task 1: 添加 ci.yml

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: 创建 workflow**

```yaml
name: CI

on:
  push:
    branches: [master, feat/v0.6]
  pull_request:
    branches: [master, feat/v0.6]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: npm

      - name: Install npm dependencies
        run: npm ci

      - name: Frontend tests
        run: npm test

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Backend tests
        run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: 本地确认命令与 CI 一致**

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: 与当前分支一致，全部通过（或已知失败需先修再合 CI）。

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: run npm test and cargo test on PR and push"
```

---

## Phase 2：runtime-cache（P1-3）

### Task 2: runtime_cache 模块 — 读写与损坏兜底

**Files:**
- Create: `src-tauri/src/runtime_cache.rs`
- Modify: `src-tauri/src/lib.rs`（`pub mod runtime_cache;`）

- [ ] **Step 1: 写失败测试（文件尚无实现时编译/断言目标行为）**

在 `src-tauri/src/runtime_cache.rs`：

```rust
use crate::models::{AppError, SkillDiscoverCache, SkillUpdateCache};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCache {
    #[serde(default = "default_runtime_cache_version")]
    pub version: u32,
    #[serde(default)]
    pub skill_discover_cache: SkillDiscoverCache,
    #[serde(default)]
    pub skill_update_cache: SkillUpdateCache,
}

fn default_runtime_cache_version() -> u32 {
    1
}

pub fn runtime_cache_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("runtime-cache.json")
}

pub fn load(app_data_dir: &Path) -> RuntimeCache {
    let path = runtime_cache_path(app_data_dir);
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => RuntimeCache::default(),
    }
}

pub fn save(app_data_dir: &Path, cache: &RuntimeCache) -> Result<(), AppError> {
    // tmp + rename；失败返回 AppError::ConfigWrite（或专用 Io）
    todo!("implement atomic write")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let cache = load(dir.path());
        assert_eq!(cache, RuntimeCache::default());
    }

    #[test]
    fn load_corrupt_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(runtime_cache_path(dir.path()), "{not-json").unwrap();
        let cache = load(dir.path());
        assert_eq!(cache, RuntimeCache::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = RuntimeCache::default();
        cache.skill_discover_cache.fetched_at = Some("2026-07-09T00:00:00Z".into());
        save(dir.path(), &cache).unwrap();
        assert_eq!(load(dir.path()), cache);
    }
}
```

- [ ] **Step 2: 实现 `save` 原子写**

模式对齐 `config_store`：写入 `runtime-cache.json.tmp`，再 rename 为 `runtime-cache.json`。Windows 上若目标存在则先 remove 再 rename（与现有 config 写法一致）。

- [ ] **Step 3: 跑测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml runtime_cache -- --nocapture
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/runtime_cache.rs src-tauri/src/lib.rs
git commit -m "feat(p1): add runtime-cache file store"
```

---

### Task 3: load 时从 config 迁移缓存

**Files:**
- Modify: `src-tauri/src/runtime_cache.rs`（增加 `migrate_from_config`）
- Modify: `src-tauri/src/config_store.rs`

- [ ] **Step 1: 写迁移测试**

```rust
#[test]
fn migrate_from_config_copies_caches_and_clears_config_fields() {
    use crate::models::{AppConfig, DiscoverableSkill, SkillDiscoverCache, SkillUpdateCache, SkillUpdateInfo};

    let dir = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default();
    config.skill_discover_cache = SkillDiscoverCache {
        fetched_at: Some("t1".into()),
        skills: vec![/* minimal DiscoverableSkill with required fields */],
    };
    config.skill_update_cache = SkillUpdateCache {
        checked_at: Some("t2".into()),
        updates: vec![],
    };

    let changed = migrate_from_config(dir.path(), &mut config).unwrap();
    assert!(changed);
    assert!(config.skill_discover_cache.skills.is_empty());
    assert!(config.skill_update_cache.updates.is_empty());
    let loaded = load(dir.path());
    assert_eq!(loaded.skill_discover_cache.fetched_at.as_deref(), Some("t1"));
}
```

（`DiscoverableSkill` 用测试里最小合法值，或 `Default` 若可用。）

- [ ] **Step 2: 实现 `migrate_from_config`**

```rust
/// 若 config 内缓存非空：覆盖写入 runtime-cache，清空 config 字段，返回 true。
pub fn migrate_from_config(app_data_dir: &Path, config: &mut AppConfig) -> Result<bool, AppError> {
    let has_discover = !config.skill_discover_cache.skills.is_empty()
        || config.skill_discover_cache.fetched_at.is_some();
    let has_updates = !config.skill_update_cache.updates.is_empty()
        || config.skill_update_cache.checked_at.is_some();
    if !has_discover && !has_updates {
        return Ok(false);
    }
    let cache = RuntimeCache {
        version: 1,
        skill_discover_cache: std::mem::take(&mut config.skill_discover_cache),
        skill_update_cache: std::mem::take(&mut config.skill_update_cache),
    };
    save(app_data_dir, &cache)?;
    Ok(true)
}
```

- [ ] **Step 3: 在 `ConfigStore::load_unlocked` 接入**

`ConfigStore` 需要知道 `app_data_dir`。当前只有 `config_path`：用 `config_path.parent()` 作为 app_data_dir。

在 `reconcile_storage_keys` 之后：

```rust
if let Some(app_data_dir) = self.config_path.parent() {
    if crate::runtime_cache::migrate_from_config(app_data_dir, &mut config)? {
        changed = true;
    }
}
```

注意：`migrate_from_config` 已写 runtime-cache；`changed = true` 会触发 `save_unlocked(config)`，此时 config 内缓存字段应已为空，**不得**再把大列表写回 config。

- [ ] **Step 4: 跑测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml runtime_cache -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml config_store -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/runtime_cache.rs src-tauri/src/config_store.rs
git commit -m "feat(p1): migrate discover/update caches out of config.json"
```

---

### Task 4: discover / updates 只持久化到 runtime-cache

**Files:**
- Modify: `src-tauri/src/commands/skill_hub.rs`
- Modify: `src-tauri/src/commands/mod.rs`（`build_app_state` / `get_app_state` 注入）
- 视需要：`skill_discover.rs` / `skill_updates.rs`（若仍写 `config.skill_*_cache`，改为返回新缓存内容，由 command 写 runtime）

**约定（锁定）：**

1. 内存中的 `AppConfig` 在请求处理期间仍可暂存 `skill_discover_cache` / `skill_update_cache`，便于现有函数签名少改。  
2. **持久化分流：**  
   - `store.save(&config)` 之前调用 `strip_runtime_caches_for_config_save(&mut config)`（清空两字段），保证磁盘 config 不含缓存。  
   - 另调用 `runtime_cache::save` 写入最新缓存。  
3. `build_app_state` / `get_app_state`：从 `runtime_cache::load(app_data_dir)` 填入返回用的 `config.skill_*_cache`（**仅内存**，随后若 save config 必须再 strip）。

- [ ] **Step 1: 增加 strip + attach 辅助**

在 `runtime_cache.rs`：

```rust
pub fn strip_from_config(config: &mut AppConfig) {
    config.skill_discover_cache = SkillDiscoverCache::default();
    config.skill_update_cache = SkillUpdateCache::default();
}

pub fn attach_to_config(app_data_dir: &Path, config: &mut AppConfig) {
    let cache = load(app_data_dir);
    config.skill_discover_cache = cache.skill_discover_cache;
    config.skill_update_cache = cache.skill_update_cache;
}

pub fn persist_from_config(app_data_dir: &Path, config: &AppConfig) -> Result<(), AppError> {
    save(
        app_data_dir,
        &RuntimeCache {
            version: 1,
            skill_discover_cache: config.skill_discover_cache.clone(),
            skill_update_cache: config.skill_update_cache.clone(),
        },
    )
}
```

- [ ] **Step 2: 改 `ConfigStore::save` 路径**

所有 `store.save(&config)` 的调用点在 save 前：

```rust
let app_data = config_path.parent().unwrap();
runtime_cache::persist_from_config(app_data, &config)?;
let mut to_save = config.clone();
runtime_cache::strip_from_config(&mut to_save);
store.save(&to_save)?;
```

更好：在 `ConfigStore::save_unlocked` 内自动 strip，并要求调用方先 `persist_from_config`——或让 `ConfigStore` 持有 app_data 并在 save 时：若 config 缓存非空则先 persist 再 strip。**推荐在 `save_unlocked` 内：**

```rust
if let Some(dir) = self.config_path.parent() {
    // 若内存 config 带了缓存，先落到 runtime-cache
    let _ = crate::runtime_cache::persist_from_config(dir, config);
    let mut stripped = config.clone();
    crate::runtime_cache::strip_from_config(&mut stripped);
    // serialize stripped ...
}
```

这样现有 `store.save(&config)` 调用点无需逐个改，且 config 磁盘永不含缓存。discover 只改内存 cache 后 `save` 仍会更新 runtime-cache。

**注意：** `persist_from_config` 每次 save 都写 runtime-cache。若 config 侧缓存已被 strip 为空，会**误把 runtime 写成空**。因此：

- **正确做法：** `save_unlocked` **只 strip、不 persist**。  
- discover/updates 命令在更新 cache 后 **显式** `runtime_cache::persist_from_config`，再 `strip` + `store.save`。  
- 普通 mutation：load 后不要 attach 缓存进要 save 的 config；或 save 前 strip，且 **不** persist。

锁定最终规则：

| 时机 | 动作 |
|------|------|
| load config | 不把 runtime 写入 config 结构（保持空） |
| build_app_state 返回前 | `attach_to_config` 到 **返回用 clone** |
| discover/checkUpdates 成功 | 更新内存 cache → `persist_from_config` → strip → `store.save`（若还需存 repos 等） |
| 普通 mutation save | strip（确保空）→ `store.save`；**不**调用 persist |

- [ ] **Step 3: 改 `discover_skills` / `check_skill_updates`**

在 `skill_hub.rs` 写缓存处之后：

```rust
let app_data = app_data_dir_from_app(&app)?;
runtime_cache::persist_from_config(&app_data, &config)?;
runtime_cache::strip_from_config(&mut config);
store.save(&config)?;
```

若原先「只为写 cache 而 save config」，可改为：**只** `persist_from_config`，仅当还有非缓存字段变更时才 `store.save`。

- [ ] **Step 4: `build_app_state` 注入**

```rust
pub fn build_app_state(
    mut config: AppConfig,
    selected_target_id: Option<String>,
    app_data_dir: Option<&Path>,
) -> Result<AppState, AppError> {
    if let Some(dir) = app_data_dir {
        runtime_cache::attach_to_config(dir, &mut config);
    }
    // existing list_skills + ...
}
```

更新所有 `build_app_state` 调用点传入 `app_data_dir`。

- [ ] **Step 5: 测试**

新增测试：模拟 config 含旧缓存 → load → 磁盘 config 无 skills 列表、runtime-cache 有数据；`build_app_state` 返回的 state 含 skills 列表。

```bash
cargo test --manifest-path src-tauri/Cargo.toml runtime_cache -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml skill_hub -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/runtime_cache.rs src-tauri/src/config_store.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/skill_hub.rs src-tauri/src/skill_discover.rs src-tauri/src/skill_updates.rs
git commit -m "feat(p1): persist discover/update caches only in runtime-cache.json"
```

---

### Task 5: README 说明 runtime-cache

**Files:**
- Modify: `README.md`
- Modify: `README.zh.md`

- [ ] **Step 1: 在 repo-cache 段落后追加**

英文：

```markdown
Discover/update result lists are stored in `{app_data}/runtime-cache.json` (separate from `config.json`). You may delete that file to clear cached lists; the next discover/update will rebuild it.
```

中文：

```markdown
发现列表与更新检查结果缓存在 `{app_data}/runtime-cache.json`（与 `config.json` 分离）。可手动删除该文件清空列表缓存；下次发现/检查更新时会重建。
```

- [ ] **Step 2: Commit**

```bash
git add README.md README.zh.md
git commit -m "docs: document runtime-cache.json"
```

---

## Phase 3：Command 精简（P1-2）

### Task 6: Full / Light 构建模式

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Test: `src-tauri/src/commands/mod.rs` 内 `#[cfg(test)]`

**锁定行为：**

```rust
pub enum AppStateBuildMode {
    /// 调用 list_skills，完整 AppState
    Full,
    /// 不调用 list_skills；使用传入的 skills 快照；仍计算 selected_target_skills
    Light { skills: Vec<SkillView> },
}

pub fn build_app_state_with_mode(
    mut config: AppConfig,
    selected_target_id: Option<String>,
    app_data_dir: Option<&Path>,
    mode: AppStateBuildMode,
) -> Result<AppState, AppError> {
    if let Some(dir) = app_data_dir {
        runtime_cache::attach_to_config(dir, &mut config);
    }
    let skills = match &mode {
        AppStateBuildMode::Full => {
            crate::skill_library::list_skills(config.settings.main_skills_dir.as_deref())?
        }
        AppStateBuildMode::Light { skills } => skills.clone(),
    };
    // selected target states 同现逻辑
    Ok(AppState { config, skills, ... })
}
```

保留 `build_app_state(...)` 作为 `Full` 的薄封装。

- [ ] **Step 1: 写测试 — Light 不扫磁盘**

用临时主库目录放 1 个 skill；先 Full 得到 skills；删掉主库目录（或改 path 使 list 会失败）；再 Light 用快照应仍成功返回相同 skills。

```rust
#[test]
fn light_build_reuses_skills_without_listing() {
    // arrange: Full once, then remove main dir
    // act: build_app_state_with_mode(..., Light { skills: snapshot })
    // assert: Ok, skills == snapshot
}
```

- [ ] **Step 2: 实现 `AppStateBuildMode` + 测试转绿**

- [ ] **Step 3: 改造 `run_with_config`**

```rust
fn run_with_config_mode<F>(
    app: tauri::AppHandle,
    mutate: F,
    selected_target_id: Option<String>,
    mode_after: impl FnOnce(&AppConfig) -> AppStateBuildMode,
) -> Result<AppState, AppErrorDto>
```

或更简单：在 mutate **之前**若走 Light，先 `list_skills` 一次取快照——**不对**，Light 的意义是 mutation 路径完全不 list。快照应来自：

- 前端不传 skills；后端 Light 时 skills=`vec![]` 且加字段？  
- **本计划锁定：** 对 install/uninstall 等，在 mutate **前**不 list；mutate 后 `Light { skills: list_skills? }` 仍会 list。  

正确做法：`run_with_config` 增加参数 `rescan_library: bool`：

- `false`：mutate 前用 `list_skills` **一次**得到 snapshot（仍有一次扫描）——仍不够。  
- 真正零扫描：mutate 后 `Light { skills: Vec::new() }` + AppState 增加 `skills_included: bool`。

**最终锁定（与 spec 一致、可测零扫描）：**

1. `AppState` 增加：

```rust
#[serde(default = "default_true")]
pub skills_included: bool,
```

`default_true` → `true`（旧客户端/旧测试兼容）。

2. Light：`skills: vec![]`，`skills_included: false`；仍填充 `config`、`selected_target_id`、`selected_target_skills`（计算 selected 时需要 skills——**用 mutate 前缓存的 snapshot**）。

3. Snapshot 获取：仅在 `rescan_library == false` 的命令里，mutate **之前**调用一次 `list_skills` 作为 snapshot，mutate 后用 snapshot 算 `selected_target_skills`，返回 `skills_included: false` 且 `skills: snapshot`（载荷仍有 skills，但 **mutate 后不二次扫描**）。  

相对现状（mutate 后 `build_app_state` 再扫一次），这是 **每次 mutation 少一次全量扫描**。若现状是「只扫一次」，收益有限——查 `run_with_config`：mutate → save → `build_app_state`（内含 list_skills）= **1 次**。  

要满足 spec「不触发主库全量扫描」，Light 必须 **0 次** `list_skills`：

- `selected_target_skills` 用 snapshot 参数：命令签名不增加；后端 Light 时若无 snapshot 则 `selected_target_skills: []` 且 `skills_included: false`，前端 merge 保留 `prev.skills` 与 `prev.selectedTargetSkills`，并在需要时用 prev.skills 本地重算——过重。  

**实用锁定（本计划采用）：**

| 命令类 | 行为 |
|--------|------|
| install_skill / uninstall_skill / 目标与项目元数据 CRUD | mutate 前 `skills = list_skills()` 一次；mutate+save；`build` 用该 snapshot，**不再** list；返回完整 skills（`skills_included: true`）但仍 **避免第二次扫描** |
| delete_main_skill / set_main_skills_dir / install_hub_skill 等 | mutate 后 `Full`（再 list 一次，因树已变） |

前端仍实现 `mergeAppState`，为阶段 4 与未来 `skills_included: false` 做准备；阶段 3 后端可先不设 `skills_included: false`，测试断言「Light 路径 list_skills 调用次数 ≤ 1」。

用可注入计数器测试：

```rust
// 在 skill_library 测试 hook 或 build_app_state_with_hooks
```

最小实现：`build_app_state_with_mode(Light { skills })` + `run_with_config` 在 save 后走 Light(snapshot)，snapshot 来自 mutate 前一次 list。

- [ ] **Step 4: 将 install/uninstall/add/update/delete target/project 改为 Light(snapshot)**

`delete_main_skill`、`set_main_skills_dir`、hub install 等保持 Full。

- [ ] **Step 5: 跑 Rust 测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/mod.rs
git commit -m "perf(p1): avoid second list_skills after non-library mutations"
```

---

### Task 7: 前端 mergeAppState

**Files:**
- Create: `src/utils/mergeAppState.ts`
- Create: `src/test/mergeAppState.test.ts`
- Modify: `src/model/types.ts`（若增加 `skillsIncluded?: boolean`）
- Modify: `src/App.tsx`（所有 `setAppState(next)` 成功路径改为 merge）

- [ ] **Step 1: 写测试**

```typescript
import { describe, expect, it } from 'vitest';
import { mergeAppState } from '../utils/mergeAppState';
import type { AppState } from '../model/types';

describe('mergeAppState', () => {
  it('keeps prev.skills when next.skillsIncluded is false', () => {
    const prev = { skills: [{ dirName: 'a' }], skillsIncluded: true } as AppState;
    const next = { skills: [], skillsIncluded: false, config: {} } as AppState;
    const merged = mergeAppState(prev, next);
    expect(merged.skills).toEqual(prev.skills);
  });

  it('replaces skills when next.skillsIncluded is true or undefined', () => {
    const prev = { skills: [{ dirName: 'a' }] } as AppState;
    const next = { skills: [{ dirName: 'b' }], config: {} } as AppState;
    expect(mergeAppState(prev, next).skills).toEqual(next.skills);
  });

  it('returns next when prev is null', () => {
    const next = { skills: [] } as AppState;
    expect(mergeAppState(null, next)).toBe(next);
  });
});
```

（按实际 `SkillView` 必填字段补全 fixture。）

- [ ] **Step 2: 实现**

```typescript
import type { AppState } from '../model/types';

export function mergeAppState(prev: AppState | null, next: AppState): AppState {
  if (!prev) return next;
  if (next.skillsIncluded === false) {
    return {
      ...next,
      skills: prev.skills,
      selectedTargetSkills:
        next.selectedTargetSkills.length > 0
          ? next.selectedTargetSkills
          : prev.selectedTargetSkills,
    };
  }
  return next;
}
```

若阶段 3 后端暂不发 `skillsIncluded: false`，merge 仍为恒等，测试覆盖未来行为。

- [ ] **Step 3: App 内替换**

```typescript
setAppState((prev) => mergeAppState(prev, next));
```

`applyAppStateSuccess` 统一走 merge。

- [ ] **Step 4: 跑前端测试**

```bash
npx vitest run src/test/mergeAppState.test.ts src/test/app.test.tsx --exclude ".worktrees/**"
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/utils/mergeAppState.ts src/test/mergeAppState.test.ts src/App.tsx src/model/types.ts src-tauri/src/models.rs
git commit -m "feat(p1): mergeAppState for light command responses"
```

---

## Phase 4：App hooks（P1-1）

### Task 8: 抽出 merge 已完成后的纯函数与 useAppDialogs

**Files:**
- Create: `src/hooks/useAppDialogs.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: 把所有 dialog 相关 useState 移入 `useAppDialogs`**

返回：`{ promptDialogOpen, setPromptDialogOpen, targetFormOpen, ... }` 等现有字段，行为不变。

- [ ] **Step 2: App 改用 hook；跑 `app.test.tsx`**

```bash
npx vitest run src/test/app.test.tsx --exclude ".worktrees/**"
```

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/hooks/useAppDialogs.ts src/App.tsx
git commit -m "refactor(p1): extract useAppDialogs from App"
```

---

### Task 9: useAppState + useAppBootstrap

**Files:**
- Create: `src/hooks/useAppState.ts`
- Create: `src/hooks/useAppBootstrap.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: `useAppState`**

托管：`appState`、`setAppState`（对外暴露 `applyRemoteState(next)` → 内部 merge）、`selectedTargetId`、`selectedProjectId`、`expandedProjectIds`、`mainView`。

- [ ] **Step 2: `useAppBootstrap`**

托管：`loading`、`error`、migration toast、初次 `getAppState`、启动 `Promise.all([discover, checkUpdates])`。依赖 `useAppState` 的 apply 方法。

- [ ] **Step 3: 跑测试并 Commit**

```bash
npx vitest run src/test/app.test.tsx --exclude ".worktrees/**"
git add src/hooks/useAppState.ts src/hooks/useAppBootstrap.ts src/App.tsx
git commit -m "refactor(p1): extract useAppState and useAppBootstrap"
```

---

### Task 10: useSkillHub — 去掉双向同步

**Files:**
- Create: `src/hooks/useSkillHub.ts`
- Create: `src/utils/hubStateFromAppState.ts`（原 `buildHubStateFromAppState` 纯函数）
- Modify: `src/App.tsx`
- Modify: `src/components/skill-hub/SkillHubPage.tsx`（若 `onHubStateChange` 仍写回 records，改为 `onSkillRecordsPatch` 或仅接受派生）

- [ ] **Step 1: 纯函数搬家 + 单测**

`hubStateFromAppState(state)` 与现 `buildHubStateFromAppState` 相同。

- [ ] **Step 2: `useSkillHub`**

- `discoverSkillsList` / `pendingUpdates` / hub 元数据  
- 从 `appState` 同步初始化（单向）  
- `applyHubState` **删除**：SkillHubPage 更新 `skillRecords` 时调用 `patchSkillRecords(records)` → 只改 `appState.config.skillRecords`  
- 派生：`hubState = useMemo(() => hubStateFromAppState(appState), [appState])` 或保留本地 validCount 等扫描字段与 appState.skills 对齐  

- [ ] **Step 3: 确认无 `applyHubState` 写 records**

```bash
rg "applyHubState|skillRecords: next.skillRecords" src/
```

Expected: 无双向写回。

- [ ] **Step 4: 跑测试并 Commit**

```bash
npx vitest run src/test --exclude ".worktrees/**"
git add src/hooks/useSkillHub.ts src/utils/hubStateFromAppState.ts src/App.tsx src/components/skill-hub/SkillHubPage.tsx
git commit -m "refactor(p1): extract useSkillHub and remove hub/app record sync"
```

---

### Task 11: useTargetActions + useProjectActions

**Files:**
- Create: `src/hooks/useTargetActions.ts`
- Create: `src/hooks/useProjectActions.ts`（若过碎可合并进 target）
- Modify: `src/App.tsx`

- [ ] **Step 1: 迁移 install/uninstall/toggle、目标 CRUD、删除主库 skill、pendingSkillKey**

统一经 `applyRemoteState(mergeAppState(...))`。

- [ ] **Step 2: 迁移项目 CRUD / 展开**

- [ ] **Step 3: App.tsx 仅组装**

目标：App 中无大段 `await installSkill` 业务；handlers 来自 hooks。

- [ ] **Step 4: 全量前端测试 + Commit**

```bash
npx vitest run src/test --exclude ".worktrees/**"
git add src/hooks/useTargetActions.ts src/hooks/useProjectActions.ts src/App.tsx
git commit -m "refactor(p1): extract target and project action hooks"
```

---

### Task 12: 整轮回归

- [ ] **Step 1: 前后端全测**

```bash
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: PASS（排除 `.worktrees` 若 npm script 仍扫到，用 vitest exclude 或修 script）。

- [ ] **Step 2: 验收清单（对照 spec）**

- [ ] CI workflow 存在且分支含 `master`、`feat/v0.6`  
- [ ] 新安装路径下 `config.json` 无大段 discover skills；存在 `runtime-cache.json`  
- [ ] 非主库 mutation 不再二次 `list_skills`  
- [ ] 无 hub↔app `skillRecords` 双向同步  
- [ ] README 已提及 runtime-cache  

- [ ] **Step 3: 若有收尾文档/注释，Commit**

```bash
git commit -m "chore(p1): final P1 regression cleanups"
```

（无改动则跳过。）

---

## Self-Review（计划对照 spec）

| Spec 项 | 对应 Task |
|---------|-----------|
| P1-4 CI | Task 1 |
| P1-3 runtime-cache 文件 | Task 2 |
| P1-3 load 迁移 | Task 3 |
| P1-3 discover 只写 runtime；内存 attach | Task 4 |
| P1-3 README | Task 5 |
| P1-2 Full/Light、少扫描 | Task 6 |
| P1-2 mergeAppState | Task 7 |
| P1-1 dialogs / bootstrap / state / hub / actions | Task 8–11 |
| 整轮验证 | Task 12 |
| 不做 P2 / 不引入 Zustand | 各阶段「明确不做」已遵守 |
| config 磁盘不写回缓存 | Task 4 规则表 |

**类型一致性：** `RuntimeCache`、`AppStateBuildMode`、`mergeAppState` / `skillsIncluded` 在 Task 2/6/7 定义，后续任务复用同一名称。

**占位符：** 无 TBD；Task 6 对「零次 vs 一次扫描」已锁定为「mutate 前最多一次 list，mutate 后零次」。

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-09-p1-ci-runtime-cache-commands-hooks.md`.

**Two execution options:**

1. **Subagent-Driven（推荐）** — 每 Task 派一个新子代理，Task 间审查，迭代快  
2. **Inline Execution** — 本会话用 executing-plans 按 Task 推进，设检查点  

**Which approach?**
