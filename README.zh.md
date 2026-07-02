# Skills Sync Manager 使用文档

[English README](README.md)

## 一、工具简介

Skills Sync Manager 是一款桌面工具，核心作用是统一维护 Skills 主库，将指定 Skills 批量同步链接至多个 Claude / 本地 Agent 目标目录，实现多个项目、Agent 共享一套 SKills。

## 二、解决的核心问题

多 Agent 目录分散维护 Skills 易出现各类问题，本工具可有效规避：

- 手动复制文件产生过期副本，版本混乱。
- 手动创建软链接后难以统一追踪、管理。
- 删除 Skill 后易残留无效链接文件。
- 手动覆盖文件，易误破坏原有目录内容。

## 三、核心功能

- **应用内更新（v0.5）**：启动时检查 GitHub Release 新版本；可选择立即更新或稍后（本次会话内不再提示，下次启动再检查）。
- **Agent / 项目目标（v0.5）**：侧边栏分为 **Agent**（用户级）与 **项目** 两级树；支持 Cursor、Claude Code、Codex 等内置 Agent 快捷添加，以及自定义路径。
- **Skill 中心（v0.4）**：在 Skill 中心浏览、安装、更新来自 GitHub、skills.sh 或自建 GitLab 仓库的 Skills。
- **自建 GitLab**：支持添加私有 GitLab 项目作为 Skill 来源；按站点配置个人访问令牌（PAT）完成认证。
- **Smart Paste（粘贴链接快速安装）**：粘贴 GitHub、skills.sh 或 GitLab 链接，预览并一键安装到主库。
- **仓库管理**：在 Skill 中心添加、启用或禁用 Skill 来源仓库；通过 **密钥管理** 维护已保存的 GitLab PAT。
- 自定义设置全局唯一的 Skills 主源目录（在 Skill 中心配置，不在侧边栏）。
- 在侧边栏 **Agent** 区域添加用户级 Agent 预设或自定义目标目录；在 **项目** 区域管理项目及其下的 Agent / 自定义目标。
- 自动校验 Skills 有效性，标识异常、待修复项目。
- 按目标目录按需安装 / 卸载指定 Skills 链接。
- 二次确认后安全删除主库 Skill，降低误删风险。
- 本地持久化保存所有配置、目录和安装记录。

## 四、下载安装

前往 [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) 下载对应系统预编译安装包：

- **Windows**：`.msi` / `.exe`
- **macOS**：`.dmg`
- **Linux**：`.AppImage` / `.deb`

> 当前安装包使用 [Sigstore](https://www.sigstore.dev/) 无密钥签名，可用 `cosign` 验证。Windows 仍可能弹出 SmartScreen 警告，macOS 仍需右键应用并选择「打开」，因为这并非操作系统原生代码签名。
>
> 验证已下载文件（以 Windows `.exe` 为例）：
>
> ```bash
> cosign verify-blob \
>   --certificate "Skills Sync Manager_x64-setup.exe.crt" \
>   --signature "Skills Sync Manager_x64-setup.exe.sig" \
>   --certificate-identity-regexp '^https://github.com/xiaoai-lazy/skills-sync-manager/\.github/workflows/release\.yml@refs/tags/v.*$' \
>   --certificate-oidc-issuer https://token.actions.githubusercontent.com \
>   "Skills Sync Manager_x64-setup.exe"
> ```
>
> 请将文件名替换为你实际下载的安装包及对应的 `.crt`/`.sig` 文件。

## 五、快速上手

按以下步骤完成基础配置：

1. 打开 Skills Sync Manager 客户端，默认进入 **Skill 中心**。
2. 在 Skill 中心配置本地 Skills 主库目录（源目录）。
3. （可选）打开 **仓库管理** 添加 GitHub、skills.sh 或 GitLab 仓库，在 **发现** 标签页浏览并安装 Skill 到主库；也可在 **Smart Paste** 粘贴链接快速安装。私有 GitLab 站点会在需要时提示配置 PAT（见下文 [GitLab 访问密钥](#gitlab-访问密钥)）。
4. 在侧边栏 **Agent** 或某个 **项目** 下添加需要同步的目标目录（可点击预设 chip 快捷添加，或使用自定义路径）。
5. 在侧边栏选中对应目标，进入目标详情页。
6. 勾选并启用需要同步的 Skills，自动完成链接部署。

侧边栏分为 **Skill 中心**（主库管理）、**Agent**（用户级目标）与 **项目**（项目级目标树）三部分。主库路径仅在 Skill 中心显示和修改，不在侧边栏展示。

有效 Skill 规范：主库目录下的子文件夹即为单个 Skill，必须包含 `SKILL.md` 文件，且文件头部 YAML 需配置 `name`、`description` 字段。

### GitLab 访问密钥

Skill 中心支持自建 GitLab 私有仓库。GitHub 与 skills.sh 等公开来源仍按原有方式使用；GitLab 私有项目需配置具备项目读权限的 PAT。

1. 在 **仓库管理** 中添加 GitLab 项目地址（HTTPS 或 SSH 风格的主机/路径）。
2. 预览或发现私有仓库中的 Skill 时，若该站点尚未保存密钥，应用会弹出 **配置 GitLab 访问密钥** 对话框。
3. 输入具备读权限的 PAT；应用会调用 GitLab API 验证，通过后写入系统凭证库（Windows 凭据管理器、macOS 钥匙串或 Linux 密钥服务）。同一 GitLab 站点下的来源仓库共用一条 PAT。
4. 在 **仓库管理** 中打开 **密钥管理**，可查看已配置站点、更新 PAT 或移除已保存密钥。

PAT 不会写入配置文件或日志。移除密钥时会从系统凭证库中删除。

## 六、安全边界（防误操作）

工具采用保守安全机制，全程规避数据风险：

- 不会主动扫描、读取整机目录，仅使用用户手动添加的目录。
- 绝不覆盖目标目录原有真实文件 / 文件夹。
- 仅卸载本工具创建的链接，不改动用户原生文件。
- 自动拦截无效 Skill，禁止违规安装部署。
- 主库 Skill 删除后不可恢复，操作需手动二次确认。

## 七、开发者说明

Tauri 命令按职责区分：

- **`install_hub_skill`**：从 Skill 中心（GitHub、skills.sh、GitLab、Smart Paste）下载或导入 Skill 到**主库**，返回更新后的 Skill 中心本地状态。
- **`install_skill`**：将主库中已有 Skill 以**符号链接 / 联接**形式安装到**目标目录**，用于目标同步，不用于 Hub 发现安装。

其他 Skill Hub 相关命令包括 `discover_skills`、`parse_smart_paste`、`check_skill_updates`、`update_skill`、`update_all_skills`。详见 `src/api/skillHub.ts` 与 `src-tauri/src/commands/skill_hub.rs`。

### v0.5 配置迁移

从 v0.4 升级时，应用会在迁移前将现有 `config.json` 复制为同目录下的 `config.json.backup-v4`（长期保留）。若备份失败，迁移将中止且不会覆盖原配置。

### 扩展 Agent 预设

可在应用数据目录创建 `agent-presets.json` 覆盖或扩展内置 Agent（按 `id` 合并）：

```json
{
  "presets": [
    {
      "id": "cursor",
      "displayName": "Cursor",
      "globalPath": "~/.cursor/skills",
      "projectRelativePath": ".cursor/skills",
      "icon": "cursor.png"
    }
  ]
}
```

`globalPath` 必填；无 `projectRelativePath` 的预设不会出现在项目级快捷添加中。自定义图标可放在 `agent-icons/` 子目录。

### 发布与更新签名（维护者）

Release CI 使用 `tauri-action` 生成 updater 产物与 `latest.json`。需在 GitHub Secrets 配置：

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（若私钥有密码）

公钥写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。应用内更新与现有 Sigstore 安装包签名并存。
