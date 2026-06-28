# 用户导向 README 优化设计

## Context

当前 `README.md` 和 `README.zh.md` 更像开发者说明：前半部分包含技术栈、脚本、测试命令，用户需要读到中段以后才看到安装信息。用户希望 README 重点面向软件使用者，突出项目目标、解决的问题和安装方法，并且英文和中文版本保持一致。

## Goal

将 README 改造成用户导向的产品介绍和安装指南：

- 首屏说明 Skills Sync Manager 是什么。
- 清楚解释它解决的痛点。
- 安装步骤前置。
- 首次使用流程清晰。
- 安全边界明确，降低用户对误删/覆盖的担忧。
- 开发者命令保留在末尾，作为辅助信息。

## Target Audience

主要读者是想下载并使用软件的人，而不是参与开发的人。

典型用户：

- 维护多个 Claude / agent skills 目录的高级用户。
- 希望将一份主 skills 库同步到多个目标目录的人。
- 不想手动复制、手动创建链接、手动追踪安装状态的人。

## README Structure

英文 `README.md` 和中文 `README.zh.md` 使用一致结构：

1. 标题与一句话定位
2. 为什么需要它
3. 它能做什么
4. 下载与安装
5. 首次使用
6. 安全边界
7. 链接行为
8. 开发者信息
9. 手动测试

## Content Design

### 1. 标题与一句话定位

保留 `Skills Sync Manager` 标题。

副标题从技术描述改为用户价值描述：

- 英文：管理一个主 skills 库，并将选中的 skills 安全同步到多个 Claude / agent 目标目录。
- 中文：统一管理一个主 skills 库，并将选中的 skills 安全同步到多个 Claude / agent 目标目录。

### 2. 为什么需要它

强调真实使用痛点：

- skills 分散在多个目标目录时难维护。
- 复制粘贴容易让不同目录版本不一致。
- 手动创建符号链接或 junction 不容易追踪。
- 删除或覆盖目录存在风险。

结论：用一个主目录作为事实来源，通过应用管理目标目录的同步状态。

### 3. 它能做什么

用用户语言描述功能，不强调内部实现：

- 设置一个主 skills 目录。
- 添加多个目标目录。
- 查看有效和无效 skill。
- 为某个目标目录安装或卸载 skill。
- 删除主库中的 skill，并在确认后清理记录过的链接。
- 持久保存设置、目标目录和安装记录。

### 4. 下载与安装

前置到 README 前半部分。

说明：

- 到 GitHub Releases 下载预编译安装包。
- Windows：`.msi` 或 `.exe`。
- macOS：`.dmg`。
- Linux：`.AppImage` 或 `.deb`。
- 当前安装包未签名，Windows/macOS 可能提示风险；这是签名状态说明，不代表安装包一定有问题。

### 5. 首次使用

写成短步骤：

1. 打开应用。
2. 设置主 skills 目录。
3. 添加一个或多个目标目录。
4. 选择目标目录。
5. 开启需要同步的 skills。

补充：主目录下每个 skill 应是直接子目录，并包含带 `name` 和 `description` frontmatter 的 `SKILL.md`。

### 6. 安全边界

保留并强化：

- 不扫描整台机器寻找 agent 目录。
- 不覆盖目标目录中已存在的真实文件或目录。
- 只卸载本应用创建并记录的链接。
- 删除主库 skill 不可恢复，需要明确确认。

### 7. 链接行为

保留现有平台差异：

- Windows 默认使用 junction。
- macOS / Linux 默认使用目录符号链接。

### 8. 开发者信息

下沉到末尾，保持简短：

- 技术栈：Tauri 2、React、TypeScript、Vite、Rust。
- 本地运行：`npm install`、`npm run tauri:dev`。
- 验证：`npm run test`、`npm run build`、`cd src-tauri && cargo test`。

### 9. 手动测试

保留现有文档链接。

## Scope

修改文件：

- `README.md`
- `README.zh.md`

不修改：

- 应用代码
- 发布 workflow
- package/version
- 测试代码

## Self-Review

- 无 TBD/TODO。
- 英文和中文 README 结构一致。
- 重点从开发命令转向用户价值、安装、首次使用和安全边界。
- 开发者内容保留但不喧宾夺主。
- 范围足够小，可以直接作为一个 README 文档改写任务实现。
