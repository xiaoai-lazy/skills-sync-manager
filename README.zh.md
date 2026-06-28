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

- 自定义设置全局唯一的 Skills 主源目录。
- 批量添加、管理多个 Claude / 本地 Agent 目标同步目录。
- 自动校验 Skills 有效性，标识异常、待修复项目。
- 按目标目录按需安装 / 卸载指定 Skills 链接。
- 二次确认后安全删除主库 Skill，降低误删风险。
- 本地持久化保存所有配置、目录和安装记录。

## 四、下载安装

前往 [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) 下载对应系统预编译安装包：

- **Windows**：`.msi` / `.exe`
- **macOS**：`.dmg`
- **Linux**：`.AppImage` / `.deb`

> 当前安装包未签名。Windows 会弹出 SmartScreen 警告，macOS 需右键应用并选择「打开」即可正常使用。后续版本将补充代码签名。

## 五、快速上手

5 步完成基础配置，开启同步使用：

1. 打开 Skills Sync Manager 客户端。
2. 配置本地 Skills 主库目录（源目录）。
3. 添加需要同步的 Agent / Claude 目标目录。
4. 在侧边栏选中对应目标目录。
5. 勾选并启用需要同步的 Skills，自动完成链接部署。

有效 Skill 规范：主库目录下的子文件夹即为单个 Skill，必须包含 `SKILL.md` 文件，且文件头部 YAML 需配置 `name`、`description` 字段。

## 六、安全边界（防误操作）

工具采用保守安全机制，全程规避数据风险：

- 不会主动扫描、读取整机目录，仅使用用户手动添加的目录。
- 绝不覆盖目标目录原有真实文件 / 文件夹。
- 仅卸载本工具创建的链接，不改动用户原生文件。
- 自动拦截无效 Skill，禁止违规安装部署。
- 主库 Skill 删除后不可恢复，操作需手动二次确认。
