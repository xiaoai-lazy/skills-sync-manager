# Skills Sync Manager

[English README](README.md)

Skills Sync Manager 是一个桌面应用，用于统一管理一个主 skills 库，并将选中的 skills 同步到多个 Claude / agent 目标目录。

当你希望只维护一份 skills，却又需要让多个本地工具目录都能使用这些 skills 时，可以使用它避免手动复制文件。

## 为什么需要它

当 skills 分散在多个 agent 目录中时，维护会很快变得混乱：

- 手动复制容易产生过期副本。
- 手动创建 symlink 或 junction 之后很难追踪。
- 删除 skill 时可能留下失效链接。
- 覆盖目标目录中已有内容可能破坏已有工作。

Skills Sync Manager 提供一个更安全的流程：把 skills 放在一个主目录中，添加你使用的目标目录，然后选择哪些 skills 要链接到每个目标目录。

## 你可以做什么

- 设置一个主 skills 目录作为源库。
- 添加多个 Claude 或其他本地 agent 的目标目录。
- 查看哪些 skills 有效，哪些需要修复。
- 为当前目标目录安装或卸载选中的 skills。
- 在明确确认后，从主库删除某个 skill。
- 在本地保存设置、目标目录和安装记录。

## 下载与安装

预编译安装包可以在 [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) 页面下载。

根据平台选择对应安装包：

- **Windows**：`.msi` 或 `.exe`
- **macOS**：`.dmg`
- **Linux**：`.AppImage` 或 `.deb`

> 当前安装包尚未签名。Windows 可能会显示 SmartScreen 警告，macOS 可能需要右键点击应用并选择“打开”。这是代码签名状态说明；后续版本会补充签名。

## 首次使用

1. 打开 Skills Sync Manager。
2. 设置主 skills 目录。
3. 添加一个或多个目标目录。
4. 从侧边栏选择一个目标目录。
5. 启用你希望安装到该目标目录的 skills。

每个 skill 应该是主 skills 目录下的直接子目录。有效 skill 需要包含 `SKILL.md` 文件，并在 YAML frontmatter 中提供 `name` 和 `description` 字段。

## 安全边界

Skills Sync Manager 的行为刻意保持保守：

- 不会扫描整台机器寻找 agent 目录。
- 不会覆盖目标目录中已经存在的真实文件或目录。
- 只会卸载由本应用创建并记录的链接。
- 会显示无效 skills，但禁止安装它们。
- 删除主库中的 skill 不可恢复，并且需要明确确认。

## 链接行为

- **Windows**：默认使用 junction。
- **macOS / Linux**：默认使用目录符号链接。

## 开发者信息

技术栈：

- Tauri 2
- React
- TypeScript
- Vite
- Rust

本地运行：

```bash
npm install
npm run tauri:dev
```

验证改动：

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## 手动测试

参见 [docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md](docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md) 获取跨平台验证清单。
