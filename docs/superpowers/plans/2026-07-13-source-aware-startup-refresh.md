# 按来源控制启动自动刷新实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标：** 启动后异步刷新配置允许的 Skill 来源，默认跳过 GitHub、刷新 GitLab 和 Skill Hub，同时保留手动全量刷新语义和失败来源的旧缓存。

**架构：** 在现有配置中增加三个来源类型开关；新增独立的启动刷新编排模块，按来源类型严格刷新发现列表和待更新列表，并在整类成功后才替换该类型缓存。现有手动 `discover_skills` 与 `check_skill_updates` 不改变语义，前端只在 `getAppState` 完成后静默触发新命令。

**技术栈：** Rust、Tauri 2、serde、React 18、TypeScript、Vitest、reqwest blocking client。

---

## 文件职责与改动范围

- `src-tauri/src/models.rs`：定义 `StartupRefreshSettings`、默认值和启动刷新返回 DTO。
- `src/model/types.ts`：声明对应 TypeScript 类型。
- `src-tauri/src/startup_refresh.rs`：来源分类、严格刷新、按类型保守合并；这是新增的唯一业务模块。
- `src-tauri/src/skill_discover.rs`：提供按 provider 严格发现仓库 Skill 的小型可复用入口。
- `src-tauri/src/skill_updates.rs`：提供按来源类型严格检查更新的入口；保留原宽松检查行为。
- `src-tauri/src/commands/skill_hub.rs`：新增读取/保存设置和启动刷新 Tauri 命令。
- `src-tauri/src/lib.rs`：注册新模块和命令。
- `src/api/skillHub.ts`：增加启动刷新和设置保存 API。
- `src/hooks/useSkillHub.ts`：封装静默启动刷新并消费合并结果。
- `src/hooks/useAppBootstrap.ts`：在本地状态加载完成后只触发一次启动刷新。
- `src/App.tsx`：传递启动刷新函数和设置到 Skill Hub 页面。
- `src/components/skill-hub/SkillHubPage.tsx`：把设置传给来源管理抽屉。
- `src/components/skill-hub/SourceManageDrawer.tsx`：显示和保存三个开关。
- `src/styles/skill-hub.css` 或现有来源管理样式文件：仅补充自动刷新设置区域样式。
- `src/test/skillHub.api.test.ts`、`src/test/app.test.tsx`、`src/test/SkillHubPage.test.tsx`：覆盖前端 API、启动和配置交互。

### Task 1：增加兼容旧配置的自动刷新设置

**文件：**
- 修改：`src-tauri/src/models.rs:218`
- 修改：`src/model/types.ts:19`
- 测试：`src-tauri/src/models.rs` 内 `#[cfg(test)]` 模块
- 测试：`src/test/appConfigFieldsAlign.test.ts`

- [ ] **Step 1：写 Rust 失败测试，固定默认值与旧配置兼容**

在 `models.rs` 测试模块增加：

```rust
#[test]
fn settings_default_startup_refresh_prefers_internal_sources() {
    let settings = Settings::default();
    assert!(!settings.startup_refresh.github);
    assert!(settings.startup_refresh.gitlab);
    assert!(settings.startup_refresh.skill_hub);
}

#[test]
fn old_settings_json_gets_startup_refresh_defaults() {
    let raw = r#"{"mainSkillsDir":null,"linkStrategy":"auto"}"#;
    let settings: Settings = serde_json::from_str(raw).expect("parse old settings");
    assert_eq!(settings.startup_refresh, StartupRefreshSettings::default());
}

#[test]
fn startup_refresh_settings_round_trip() {
    let settings = Settings {
        startup_refresh: StartupRefreshSettings {
            github: true,
            gitlab: false,
            skill_hub: true,
        },
        ..Settings::default()
    };
    let raw = serde_json::to_string(&settings).expect("serialize settings");
    let restored: Settings = serde_json::from_str(&raw).expect("deserialize settings");
    assert_eq!(restored, settings);
}
```

- [ ] **Step 2：运行测试确认失败**

运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml startup_refresh
```

预期：编译失败，提示 `StartupRefreshSettings` 或 `startup_refresh` 不存在。

- [ ] **Step 3：实现 Rust 配置类型和返回 DTO**

在 `models.rs` 中加入：

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupRefreshSettings {
    #[serde(default)]
    pub github: bool,
    #[serde(default = "default_true")]
    pub gitlab: bool,
    #[serde(default = "default_true")]
    pub skill_hub: bool,
}

fn default_true() -> bool {
    true
}

impl Default for StartupRefreshSettings {
    fn default() -> Self {
        Self {
            github: false,
            gitlab: true,
            skill_hub: true,
        }
    }
}
```

给 `Settings` 增加：

```rust
#[serde(default)]
pub startup_refresh: StartupRefreshSettings,
```

并在 `Settings::default()` 中设置 `StartupRefreshSettings::default()`。

同时定义启动返回值：

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupSkillRefreshResult {
    pub discover_skills: Vec<DiscoverableSkill>,
    pub pending_updates: Vec<SkillUpdateInfo>,
    pub warnings: Vec<String>,
}
```

- [ ] **Step 4：同步 TypeScript 类型并更新字段对齐测试**

在 `src/model/types.ts` 增加：

```typescript
export interface StartupRefreshSettings {
  github: boolean;
  gitlab: boolean;
  skillHub: boolean;
}

export interface Settings {
  mainSkillsDir: string | null;
  linkStrategy: LinkStrategy;
  startupRefresh: StartupRefreshSettings;
}

export interface StartupSkillRefreshResult {
  discoverSkills: DiscoverableSkill[];
  pendingUpdates: SkillUpdateInfo[];
  warnings: string[];
}
```

将测试 fixture 中的 `settings` 统一补上：

```typescript
startupRefresh: { github: false, gitlab: true, skillHub: true }
```

- [ ] **Step 5：运行模型和前端类型测试**

运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml models::tests
npm test -- --run src/test/appConfigFieldsAlign.test.ts
npm run build
```

预期：全部退出码为 `0`。

- [ ] **Step 6：提交配置模型**

```powershell
git add -- src-tauri/src/models.rs src/model/types.ts src/test
git commit -m "feat(settings): add startup refresh source defaults"
```

### Task 2：建立来源分类与保守缓存合并单元

**文件：**
- 新建：`src-tauri/src/startup_refresh.rs`
- 修改：`src-tauri/src/lib.rs`
- 测试：`src-tauri/src/startup_refresh.rs` 内 `#[cfg(test)]` 模块

- [ ] **Step 1：写失败测试固定来源分类**

测试构造 GitHub、GitLab、Skill Hub 三种 `DiscoverableSkill` 和 `SkillUpdateInfo + SkillRecord`，断言：

```rust
#[test]
fn discover_source_kind_uses_provider_and_skillhub_marker() {
    assert_eq!(discover_source_kind(&github_skill()), SourceKind::Github);
    assert_eq!(discover_source_kind(&gitlab_skill()), SourceKind::Gitlab);
    assert_eq!(discover_source_kind(&hub_skill()), SourceKind::SkillHub);
}

#[test]
fn update_source_kind_uses_storage_key_record() {
    let config = config_with_three_source_records();
    assert_eq!(update_source_kind(&config, &github_update()), Some(SourceKind::Github));
    assert_eq!(update_source_kind(&config, &gitlab_update()), Some(SourceKind::Gitlab));
    assert_eq!(update_source_kind(&config, &hub_update()), Some(SourceKind::SkillHub));
}
```

- [ ] **Step 2：写失败测试固定缓存替换和保留规则**

```rust
#[test]
fn merge_discover_kind_replaces_only_selected_kind() {
    let old = vec![github_skill(), gitlab_skill(), hub_skill()];
    let merged = merge_discover_kind(old, SourceKind::Gitlab, vec![new_gitlab_skill()]);
    assert_eq!(source_keys(&merged), vec!["github-old", "gitlab-new", "hub-old"]);
}

#[test]
fn merge_update_kind_replaces_only_selected_kind() {
    let config = config_with_three_source_records();
    let old = vec![github_update(), gitlab_update(), hub_update()];
    let merged = merge_update_kind(&config, old, SourceKind::SkillHub, vec![new_hub_update()]);
    assert_eq!(update_keys(&merged), vec!["github-old", "gitlab-old", "hub-new"]);
}
```

- [ ] **Step 3：运行测试确认模块尚不存在**

运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml startup_refresh::tests
```

预期：编译失败或无对应模块。

- [ ] **Step 4：实现最小分类和合并函数**

新增模块核心接口：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Github,
    Gitlab,
    SkillHub,
}

pub fn discover_source_kind(skill: &DiscoverableSkill) -> SourceKind {
    if skill.source == "skillhub" {
        SourceKind::SkillHub
    } else if skill.source == "gitlab" || skill.repo_host != default_github_host() {
        SourceKind::Gitlab
    } else {
        SourceKind::Github
    }
}

pub fn merge_discover_kind(
    old: Vec<DiscoverableSkill>,
    kind: SourceKind,
    fresh: Vec<DiscoverableSkill>,
) -> Vec<DiscoverableSkill> {
    let mut merged = old
        .into_iter()
        .filter(|skill| discover_source_kind(skill) != kind)
        .collect::<Vec<_>>();
    merged.extend(fresh);
    deduplicate_discoverable_skills(merged)
}
```

更新分类必须通过 `storage_key` 查找 `SkillRecord`；无法分类的旧条目一律保留：

```rust
pub fn update_source_kind(config: &AppConfig, update: &SkillUpdateInfo) -> Option<SourceKind>;
pub fn merge_update_kind(
    config: &AppConfig,
    old: Vec<SkillUpdateInfo>,
    kind: SourceKind,
    fresh: Vec<SkillUpdateInfo>,
) -> Vec<SkillUpdateInfo>;
```

- [ ] **Step 5：注册模块并运行测试**

在 `src-tauri/src/lib.rs` 增加：

```rust
mod startup_refresh;
```

运行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml startup_refresh::tests
```

预期：全部通过。

- [ ] **Step 6：提交纯函数单元**

```powershell
git add -- src-tauri/src/startup_refresh.rs src-tauri/src/lib.rs
git commit -m "feat(updates): add source-aware cache merging"
```

### Task 3：实现严格的按来源启动刷新后端

**文件：**
- 修改：`src-tauri/src/skill_discover.rs:10`
- 修改：`src-tauri/src/skill_updates.rs:96`
- 修改：`src-tauri/src/startup_refresh.rs`
- 修改：`src-tauri/src/commands/skill_hub.rs:1`
- 修改：`src-tauri/src/lib.rs:45`
- 测试：上述三个 Rust 模块内测试

- [ ] **Step 1：写严格发现失败测试**

为 `skill_discover.rs` 增加带 hook 的 provider 过滤入口测试：

```rust
#[test]
fn strict_discover_repos_calls_only_selected_provider() {
    let config = config_with_github_and_gitlab_repos();
    let calls = RefCell::new(Vec::new());
    let result = discover_repos_strict_with_hook(
        &config,
        None,
        "gitlab",
        |repo| {
            calls.borrow_mut().push(repo.provider.clone());
            Ok(vec![discoverable_for(repo)])
        },
    ).expect("gitlab discovery");
    assert_eq!(&*calls.borrow(), &["gitlab"]);
    assert_eq!(result.len(), 1);
}

#[test]
fn strict_discover_repos_fails_entire_kind_when_one_repo_fails() {
    let config = config_with_two_gitlab_repos();
    let result = discover_repos_strict_with_hook(&config, None, "gitlab", |repo| {
        if repo.name == "broken" { Err(sample_error()) } else { Ok(vec![discoverable_for(repo)]) }
    });
    assert!(result.is_err());
}
```

- [ ] **Step 2：实现严格仓库发现入口**

增加：

```rust
pub fn discover_repos_strict(
    config: &AppConfig,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
    provider: &str,
) -> Result<Vec<DiscoverableSkill>, AppError>;
```

它只遍历 `enabled && repo.provider == provider` 的仓库；任一仓库失败立即返回 `Err`，不写缓存。生产实现复用 `discover_repo_skills(repo, main_dir, app_data_dir, false)`。

- [ ] **Step 3：写严格更新检查失败测试**

为 `skill_updates.rs` 增加：

```rust
#[test]
fn strict_repo_update_check_filters_provider() {
    let mut config = config_with_github_and_gitlab_records();
    let calls = RefCell::new(Vec::new());
    let updates = check_repo_updates_strict_with_hook(
        &mut config,
        main_dir(),
        "gitlab",
        |repo| {
            calls.borrow_mut().push(repo.provider.clone());
            Ok(repo_fixture(repo))
        },
    ).expect("gitlab updates");
    assert_eq!(&*calls.borrow(), &["gitlab"]);
    assert!(updates.iter().all(|item| item.storage_key.contains("gitlab")));
}

#[test]
fn strict_hub_update_check_propagates_endpoint_failure() {
    let mut config = config_with_hub_records();
    let result = check_hub_updates_strict_with_hooks(
        &mut config,
        main_dir(),
        |_, _| Err(sample_error()),
        |_, _, _| Err(sample_error()),
    );
    assert!(result.is_err());
}
```

- [ ] **Step 4：实现严格更新入口但保留原宽松 API**

增加生产接口：

```rust
pub fn check_repo_updates_strict(
    config: &mut AppConfig,
    main_dir: &Path,
    provider: &str,
) -> Result<Vec<SkillUpdateInfo>, AppError>;

pub fn check_hub_updates_strict(
    config: &mut AppConfig,
    main_dir: &Path,
) -> Result<Vec<SkillUpdateInfo>, AppError>;
```

严格入口遇到下载、列表、归档或哈希错误时返回 `Err`。现有 `check_updates` 继续调用宽松路径并保持“单个来源失败时跳过”的手动检查行为，不改变其签名和测试。

- [ ] **Step 5：写启动编排失败测试**

在 `startup_refresh.rs` 用注入 closure 测试：

```rust
#[test]
fn refresh_enabled_kinds_skips_github_by_default() {
    let mut config = config_with_cached_three_kinds();
    let calls = RefCell::new(Vec::new());
    let result = refresh_with_hooks(&mut config, |kind| {
        calls.borrow_mut().push(kind);
        Ok(fresh_for(kind))
    });
    assert_eq!(&*calls.borrow(), &[SourceKind::Gitlab, SourceKind::SkillHub]);
    assert!(result.warnings.is_empty());
}

#[test]
fn failed_kind_keeps_both_old_caches() {
    let mut config = config_with_cached_three_kinds();
    let before = config.clone();
    let result = refresh_with_hooks(&mut config, |kind| {
        if kind == SourceKind::Gitlab { Err("gitlab unavailable".into()) } else { Ok(fresh_for(kind)) }
    });
    assert_eq!(gitlab_discover(&config), gitlab_discover(&before));
    assert_eq!(gitlab_updates(&config), gitlab_updates(&before));
    assert_eq!(result.warnings.len(), 1);
}
```

- [ ] **Step 6：实现启动编排和 Tauri 命令**

`startup_refresh.rs` 暴露：

```rust
pub fn refresh_enabled_sources(
    config: &mut AppConfig,
    main_dir: Option<&Path>,
    app_data_dir: &Path,
) -> StartupSkillRefreshResult;
```

每个启用类型先在临时结果中完成发现和更新检查；两者均成功才调用两个 merge 函数。失败只追加脱敏警告，不修改该类型缓存。

`commands/skill_hub.rs` 新增异步命令：

```rust
#[tauri::command]
pub async fn refresh_startup_skill_sources(
    app: AppHandle,
) -> Result<StartupSkillRefreshResult, AppErrorDto>;
```

命令步骤必须是：加载 config、挂载 runtime cache、`spawn_blocking` 执行刷新、重新加载最新 config、只合并 `skill_discover_cache` 与 `skill_update_cache`、持久化 runtime cache、保存剥离缓存后的 config。使用现有 discover/update guard，重叠时返回进行中错误。

在 `lib.rs` 注册 `refresh_startup_skill_sources`。

- [ ] **Step 7：运行 Rust 定向与完整测试**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml startup_refresh::tests
cargo test --manifest-path src-tauri/Cargo.toml skill_discover::tests
cargo test --manifest-path src-tauri/Cargo.toml skill_updates::tests
cargo test --manifest-path src-tauri/Cargo.toml
```

预期：全部通过；若凭据存储并行隔离测试偶发失败，立即单独重跑该精确测试并记录，不能忽略其他失败。

- [ ] **Step 8：提交后端启动刷新**

```powershell
git add -- src-tauri/src/startup_refresh.rs src-tauri/src/skill_discover.rs src-tauri/src/skill_updates.rs src-tauri/src/commands/skill_hub.rs src-tauri/src/lib.rs
git commit -m "feat(updates): refresh configured sources at startup"
```

### Task 4：增加设置保存命令与前端 API

**文件：**
- 修改：`src-tauri/src/commands/skill_hub.rs`
- 修改：`src-tauri/src/lib.rs`
- 修改：`src/api/skillHub.ts`
- 测试：`src/test/skillHub.api.test.ts`
- 测试：`src-tauri/src/commands/skill_hub.rs` 测试模块

- [ ] **Step 1：写前端 API 失败测试**

```typescript
it('refreshStartupSkillSources invokes the startup-only command', async () => {
  invokeMock.mockResolvedValue({ discoverSkills: [], pendingUpdates: [], warnings: [] });
  await refreshStartupSkillSources();
  expect(invokeMock).toHaveBeenCalledWith('refresh_startup_skill_sources');
});

it('setStartupRefreshSettings sends all source switches', async () => {
  const settings = { github: true, gitlab: false, skillHub: true };
  invokeMock.mockResolvedValue(settings);
  await setStartupRefreshSettings(settings);
  expect(invokeMock).toHaveBeenCalledWith('set_startup_refresh_settings', { settings });
});
```

- [ ] **Step 2：运行 API 测试确认失败**

```powershell
npm test -- --run src/test/skillHub.api.test.ts
```

预期：导入或函数不存在。

- [ ] **Step 3：实现 Rust 设置保存命令**

```rust
#[tauri::command]
pub fn set_startup_refresh_settings(
    app: AppHandle,
    settings: StartupRefreshSettings,
) -> Result<StartupRefreshSettings, AppErrorDto> {
    let store = store_from_app(&app).map_err(|err| err.to_dto())?;
    let mut config = store.load().map_err(|err| err.to_dto())?;
    config.settings.startup_refresh = settings.clone();
    store.save(&config).map_err(|err| err.to_dto())?;
    Ok(settings)
}
```

增加单元测试验证只修改设置字段，不改变 targets、repos 或 records。注册命令到 `lib.rs`。

- [ ] **Step 4：实现 TypeScript API**

```typescript
export async function refreshStartupSkillSources(): Promise<StartupSkillRefreshResult> {
  return invoke<StartupSkillRefreshResult>('refresh_startup_skill_sources');
}

export async function setStartupRefreshSettings(
  settings: StartupRefreshSettings,
): Promise<StartupRefreshSettings> {
  return invoke<StartupRefreshSettings>('set_startup_refresh_settings', { settings });
}
```

- [ ] **Step 5：运行前后端定向测试并提交**

```powershell
npm test -- --run src/test/skillHub.api.test.ts
cargo test --manifest-path src-tauri/Cargo.toml commands::skill_hub::tests
git add -- src-tauri/src/commands/skill_hub.rs src-tauri/src/lib.rs src/api/skillHub.ts src/test/skillHub.api.test.ts
git commit -m "feat(settings): persist startup refresh switches"
```

### Task 5：接入静默启动刷新

**文件：**
- 修改：`src/hooks/useSkillHub.ts`
- 修改：`src/hooks/useAppBootstrap.ts`
- 修改：`src/App.tsx`
- 测试：`src/test/app.test.tsx`

- [ ] **Step 1：替换启动行为测试**

把原测试 `runs background discover and check updates on startup` 改为：

```typescript
it('loads cached state then runs the source-aware startup refresh', async () => {
  vi.mocked(refreshStartupSkillSources).mockResolvedValue({
    discoverSkills: [gitlabDiscoverable],
    pendingUpdates: [hubUpdate],
    warnings: [],
  });

  render(<App />);
  await screen.findByRole('heading', { name: 'Skill 中心' });

  await waitFor(() => expect(refreshStartupSkillSources).toHaveBeenCalledTimes(1));
  expect(discoverSkills).not.toHaveBeenCalled();
  expect(checkSkillUpdates).not.toHaveBeenCalled();
});

it('keeps cached state and stays silent when startup refresh fails', async () => {
  vi.mocked(refreshStartupSkillSources).mockRejectedValue(new Error('offline'));
  render(<App />);
  await screen.findByRole('heading', { name: 'Skill 中心' });
  expect(screen.queryByText('offline')).not.toBeInTheDocument();
});
```

- [ ] **Step 2：运行测试确认失败**

```powershell
npm test -- --run src/test/app.test.tsx
```

预期：启动刷新 mock 或调用不存在。

- [ ] **Step 3：在 `useSkillHub` 实现静默刷新函数**

```typescript
const startupRefreshInFlight = useRef(false);

const runStartupRefresh = useCallback(async (): Promise<void> => {
  if (startupRefreshInFlight.current) return;
  startupRefreshInFlight.current = true;
  try {
    const result = await refreshStartupSkillSources();
    setDiscoverSkillsList(result.discoverSkills);
    setPendingUpdates(result.pendingUpdates);
    await refreshHub();
  } catch {
    // Startup refresh is best-effort; cached state remains authoritative.
  } finally {
    startupRefreshInFlight.current = false;
  }
}, [refreshHub]);
```

保留现有 `runBackgroundDiscover` 和 `runBackgroundCheckUpdates` 供其他调用者使用；若搜索确认只有启动使用，则删除这两个包装函数，但不得删除 `discoverSkills` 和 `checkSkillUpdates` 的手动 UI 调用。

- [ ] **Step 4：修改 `useAppBootstrap` 只触发新函数**

参数改为：

```typescript
runStartupRefresh: () => Promise<void>;
```

启动 effect：

```typescript
useEffect(() => {
  if (!appState || startupBackgroundDone.current) return;
  startupBackgroundDone.current = true;
  void runStartupRefresh();
}, [appState, runStartupRefresh]);
```

`App.tsx` 只传入 `runStartupRefresh`。

- [ ] **Step 5：运行前端测试和构建并提交**

```powershell
npm test -- --run src/test/app.test.tsx
npm test -- --run
npm run build
git add -- src/hooks/useSkillHub.ts src/hooks/useAppBootstrap.ts src/App.tsx src/test/app.test.tsx
git commit -m "feat(startup): refresh internal skill sources silently"
```

### Task 6：在来源管理中增加三个开关

**文件：**
- 修改：`src/components/skill-hub/SkillHubPage.tsx`
- 修改：`src/components/skill-hub/SourceManageDrawer.tsx`
- 修改：`src/App.tsx`
- 修改：现有来源管理样式文件（通过 `rg -n "source-manage-drawer" src/styles` 确定）
- 测试：`src/test/SkillHubPage.test.tsx`

- [ ] **Step 1：写开关交互失败测试**

```typescript
it('shows startup refresh defaults and persists a changed switch', async () => {
  const user = userEvent.setup();
  vi.mocked(setStartupRefreshSettings).mockResolvedValue({
    github: true,
    gitlab: true,
    skillHub: true,
  });
  renderHub({
    startupRefreshSettings: { github: false, gitlab: true, skillHub: true },
  });

  await user.click(screen.getByRole('button', { name: '来源管理' }));
  expect(screen.getByRole('checkbox', { name: 'GitHub 启动自动刷新' })).not.toBeChecked();
  expect(screen.getByRole('checkbox', { name: 'GitLab 启动自动刷新' })).toBeChecked();
  expect(screen.getByRole('checkbox', { name: 'Skill Hub 启动自动刷新' })).toBeChecked();

  await user.click(screen.getByRole('checkbox', { name: 'GitHub 启动自动刷新' }));
  expect(setStartupRefreshSettings).toHaveBeenCalledWith({
    github: true,
    gitlab: true,
    skillHub: true,
  });
});
```

再增加保存失败测试：API reject 后 checkbox 恢复原值并调用 `onError`。

- [ ] **Step 2：运行组件测试确认失败**

```powershell
npm test -- --run src/test/SkillHubPage.test.tsx
```

预期：新 prop、checkbox 或 API 不存在。

- [ ] **Step 3：传递设置并实现抽屉状态**

给 `SkillHubPageProps` 和 `SourceManageDrawerProps` 增加：

```typescript
startupRefreshSettings: StartupRefreshSettings;
onStartupRefreshSettingsChange?: (settings: StartupRefreshSettings) => void;
```

抽屉中维护 `savingStartupRefresh`，切换时先构造 next，成功后更新并回调，失败时保持 previous：

```typescript
const handleStartupRefreshToggle = async (
  key: keyof StartupRefreshSettings,
  checked: boolean,
) => {
  const previous = startupRefreshSettings;
  const next = { ...previous, [key]: checked };
  setLocalStartupRefresh(next);
  try {
    const saved = await setStartupRefreshSettings(next);
    setLocalStartupRefresh(saved);
    onStartupRefreshSettingsChange?.(saved);
  } catch (err) {
    setLocalStartupRefresh(previous);
    onError?.(errorMessage(err));
  }
};
```

- [ ] **Step 4：增加中文配置区域**

在来源列表上方加入：

```tsx
<section className="startup-refresh-settings" aria-labelledby="startup-refresh-title">
  <h3 id="startup-refresh-title">启动自动刷新</h3>
  <p>仅影响应用启动时的后台刷新；手动刷新始终检查所有已启用来源。</p>
  {([
    ['github', 'GitHub'],
    ['gitlab', 'GitLab'],
    ['skillHub', 'Skill Hub'],
  ] as const).map(([key, label]) => (
    <label key={key} className="startup-refresh-option">
      <span>{label}</span>
      <input
        type="checkbox"
        checked={localStartupRefresh[key]}
        disabled={savingStartupRefresh}
        aria-label={`${label} 启动自动刷新`}
        onChange={(event) => void handleStartupRefreshToggle(key, event.target.checked)}
      />
    </label>
  ))}
</section>
```

`App.tsx` 从 `appState.config.settings.startupRefresh` 传入，并在保存成功回调中只更新本地 `appState.config.settings.startupRefresh`。

- [ ] **Step 5：补充局部样式并验证**

样式只定义该区域的边框、间距、行布局和说明文字颜色，复用现有 switch 控件类，不创建新的全局设计系统。

运行：

```powershell
npm test -- --run src/test/SkillHubPage.test.tsx
npm test -- --run
npm run build
```

预期：全部通过。

- [ ] **Step 6：提交来源管理 UI**

```powershell
git add -- src/App.tsx src/components/skill-hub/SkillHubPage.tsx src/components/skill-hub/SourceManageDrawer.tsx src/styles src/test/SkillHubPage.test.tsx
git commit -m "feat(settings): configure startup refresh by source"
```

### Task 7：完整验证与交付检查

**文件：**
- 检查：全部本次修改文件
- 可选修改：仅修复本次测试暴露的直接回归

- [ ] **Step 1：运行格式与差异检查**

```powershell
git diff --check
git status --short
```

预期：`git diff --check` 无输出；状态只包含本任务预期文件。

- [ ] **Step 2：运行完整前端验证**

```powershell
npm test -- --run
npm run build
```

预期：所有 Vitest 测试通过，TypeScript 与 Vite 构建退出码为 `0`。

- [ ] **Step 3：运行完整 Rust 验证**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

预期：全部通过。若只有已知 `credential_store::tests::reconcile_gitlab_credential_hosts_registers_hosts_with_tokens` 并行隔离失败，执行：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml credential_store::tests::reconcile_gitlab_credential_hosts_registers_hosts_with_tokens -- --exact --nocapture
```

只有单独运行通过且无其他失败时，才记录为既有测试隔离风险。

- [ ] **Step 4：人工核对关键行为**

检查：

```text
1. 默认设置显示 GitHub 关闭、GitLab/Skill Hub 开启。
2. 启动页面先显示缓存，不等待后台任务。
3. 启动默认不触发 GitHub 仓库请求。
4. GitLab 或 Skill Hub 不可用时页面不弹启动错误，旧缓存保留。
5. 手动刷新仍检查 GitHub、GitLab 和 Skill Hub 全部已启用来源。
6. 应用版本自动检查仍执行。
```

- [ ] **Step 5：提交必要的最终修正（若有）**

仅当验证产生直接修正时，先用 `git diff --name-only` 核对文件，再逐个显式加入本任务文件。例如前端启动测试需要修正时执行：

```powershell
git diff --name-only
git add -- src/hooks/useAppBootstrap.ts src/hooks/useSkillHub.ts src/test/app.test.tsx
git commit -m "test: verify source-aware startup refresh"
```

- [ ] **Step 6：汇报分支状态**

```powershell
git status --short --branch
git log --oneline master..HEAD
```

预期：工作区干净；提交序列按配置、缓存合并、后端刷新、API、启动接入、UI 的边界排列。
