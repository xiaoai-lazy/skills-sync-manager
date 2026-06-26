# Skills Sync Manager

一个跨平台桌面应用，用于管理一个主 skills 目录，并通过目录链接将选中的 skills 同步到多个目标目录。

## 应用功能

- 管理一个可配置的主 skills 目录。
- 校验主目录下的直接子目录作为 skill。
- skill 有效当且仅当其目录包含 `SKILL.md`，且 YAML frontmatter 中带有 `name` 和 `description`。
- 显示无效 skill 目录及具体校验错误，但禁止安装。
- 为选中的目标目录即时安装有效 skill。
- 从选中的目标目录即时卸载已安装的 skill。
- 将设置、目标目录和安装记录持久化到本地应用数据 JSON 文件。
- 在确认后直接从主目录删除 skill，并先清理所有记录的目标链接。

## 应用不会做的事

- 不会扫描机器上的 agent 目录。
- 不会覆盖目标目录中已存在的内容。
- 只卸载由本应用创建并记录的链接。
- 主目录 skill 删除在 v1 中不可恢复，需要确认。

## 技术栈

- Tauri 2
- React
- TypeScript
- Vite
- Rust

## 脚本

- `npm run dev` 启动 Vite 前端开发服务器。
- `npm run build` 类型检查并构建前端。
- `npm run test` 使用 Vitest 运行前端测试。
- `npm run tauri` 运行 Tauri CLI。
- `npm run tauri:dev` 以开发模式启动 Tauri 桌面应用。

## 开发

```bash
npm install
npm run tauri:dev
```

## 测试

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## 后端检查

在 Tauri crate 目录中运行 Rust 检查：

```bash
cd src-tauri
cargo test
```

## 下载与安装

预编译的安装包可以在 [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) 页面下载。

按平台选择对应的安装包：

- **Windows**：`.msi` 或 `.exe`
- **macOS**：`.dmg`
- **Linux**：`.AppImage` 或 `.deb`

> 当前安装包**未签名**。Windows 可能会显示 SmartScreen 警告，macOS 可能需要右键 → 打开。后续版本会补充代码签名。

要手动发布新版本，可以打一个 tag 并推送：

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动构建各平台安装包，并创建一个草稿 Release。

## 链接行为

- Windows：默认使用 junction。
- macOS / Linux：默认使用目录符号链接。

## 警告

主目录 skill 删除在 v1 中不可恢复。应用会在删除源 skill 目录之前，先移除已记录的目标链接。

## 手动测试

参见 [docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md](docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md) 获取跨平台验证清单。
