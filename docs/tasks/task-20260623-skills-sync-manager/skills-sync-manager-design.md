# 任务设计 — Skills Sync Manager

- **任务 ID**：task-20260623-skills-sync-manager
- **关联需求**：个人本机 skills 管理桌面应用
- **状态**：草稿
- **文档日期**：2026-06-23

---

## 1. 设计目标

第一版是一个个人本机使用的桌面应用，用来统一管理一个主 skills 目录和多个目标安装目录。应用不扫描全盘目录，不自动接管已有内容，也不做团队同步或多主目录管理。

核心目标：

- 让用户手动指定一个主 skills 目录。
- 让用户手动维护多个目标目录。
- 让用户在目标目录维度选择要安装的 skills。
- 通过目录链接把主目录中的 skill 同步到各个目标目录。
- 只删除本软件创建并记录过的链接，避免误伤用户已有目录。

---

## 2. 产品范围

### 2.1 本期包含

- 单一主 skills 目录配置。
- 目标目录的新增、编辑、删除。
- 主目录下一级子目录的 skill 识别与校验。
- 基于 `SKILL.md` frontmatter 的有效性判断。
- 以目标为中心的安装 / 卸载交互。
- 即时生效的链接创建与删除。
- 主目录 skill 的直接删除。
- 安装记录持久化到本机 JSON 文件。
- Windows 默认 junction，非 Windows 默认 symlink。

### 2.2 本期不包含

- 自动扫描全盘 Agent / 项目目录。
- 多主目录管理。
- 团队共享仓库。
- 批量预览 / 待执行队列。
- 复制目录代替链接。
- 回收站 / 备份区 / 恢复功能。
- 文件级 skill 管理。
- 自动修复外部手工改动。

---

## 3. 交互设计

### 3.1 页面结构

左侧分两块：

- **主目录**
  - 当前主 skills 目录路径
  - 刷新按钮
  - 打开目录按钮
  - 有效 / 无效 skill 列表入口
- **目标目录**
  - 目标列表
  - 新增目标
  - 编辑目标
  - 删除目标

主内容区根据选中的目标显示：

- 目标名称与 skills 安装目录
- 主目录中所有有效 skills
- 每个 skill 的名称、描述、目录名、状态
- 安装 / 已安装状态切换
- 冲突或异常提示

### 3.2 操作模式

- 勾选 skill：立即安装到当前目标目录。
- 取消勾选 skill：立即卸载当前目标目录中的本软件链接。
- 主目录删除 skill：二次确认后立即执行删除前清理，再直接删除真实 skill。

### 3.3 状态展示

每个 skill 在目标上下文中显示以下状态之一：

- `未安装`
- `已安装`
- `冲突`
- `缺失`
- `异常`

---

## 4. 架构设计

### 4.1 分层

本期采用三层结构：

1. **UI 层**：只负责展示和触发用户操作，不直接处理文件系统。
2. **核心服务层**：负责配置、校验、安装、卸载、删除等业务规则。
3. **文件系统适配层**：封装 Windows / 非 Windows 的链接差异和文件操作细节。

### 4.2 核心模块

#### Config Store

职责：

- 读取和写入 JSON 配置。
- 保存主目录路径、目标列表、安装记录。
- 提供配置版本字段，便于以后迁移。

#### Skill Library

职责：

- 读取主目录下一级子目录。
- 校验每个目录是否包含 `SKILL.md`。
- 校验 `SKILL.md` frontmatter 是否包含 `name` 和 `description`。
- 输出有效 skill 与无效 skill 列表。

#### Target Registry

职责：

- 管理目标名称与目标目录路径。
- 校验目标路径是否存在、是否可写。
- 维护目标的稳定 ID。

#### Link Installer

职责：

- 安装 skill 到目标目录。
- 根据平台策略创建 junction 或 symlink。
- 卸载时只删除本软件创建并记录过的链接。
- 在安装前做冲突检测。

#### Skill Remover

职责：

- 删除主目录中的真实 skill。
- 删除前先清理所有指向该 skill 的已记录链接。
- 任一链接清理失败则中止主目录删除。

#### File System Adapter

职责：

- 统一封装路径存在性检查、目录检查、链接创建、链接删除、真实目录删除。
- Windows 默认使用 junction。
- 非 Windows 默认使用 symlink。

---

## 5. 数据模型

### 5.1 配置文件

建议使用一个本机 JSON 文件保存全部状态，逻辑上拆成三组数据：

- `settings`
- `targets`
- `installations`

### 5.2 Settings

```json
{
  "version": 1,
  "settings": {
    "mainSkillsDir": "C:/Users/zxxk/.skills-library",
    "linkStrategy": "auto"
  }
}
```

说明：

- `mainSkillsDir`：唯一主 skills 目录。
- `linkStrategy`：内部策略字段，`auto` 表示 Windows 用 junction，其他平台用 symlink。

### 5.3 Targets

```json
{
  "targets": [
    {
      "id": "target_claude_global",
      "name": "Claude Code Global Skills",
      "skillsDir": "C:/Users/zxxk/.claude/skills",
      "createdAt": "2026-06-23T10:00:00Z",
      "updatedAt": "2026-06-23T10:00:00Z"
    }
  ]
}
```

说明：

- `id`：稳定 ID。
- `name`：界面显示名称。
- `skillsDir`：目标的安装目录。
- `createdAt` / `updatedAt`：辅助排序与排查。

### 5.4 Installations

```json
{
  "installations": [
    {
      "id": "inst_001",
      "skillDirName": "brainstorming",
      "skillName": "brainstorming",
      "sourcePath": "C:/Users/zxxk/.skills-library/brainstorming",
      "targetId": "target_claude_global",
      "linkPath": "C:/Users/zxxk/.claude/skills/brainstorming",
      "linkType": "junction",
      "createdAt": "2026-06-23T10:05:00Z"
    }
  ]
}
```

说明：

- `skillDirName`：主目录一级目录名。
- `skillName`：用于展示的 skill 名称。
- `sourcePath`：创建链接时的主目录路径。
- `targetId`：关联目标。
- `linkPath`：实际链接路径。
- `linkType`：`junction` 或 `symlink`。
- `createdAt`：安装时间。

### 5.5 运行时 skill 视图模型

运行时从文件系统生成，不单独持久化：

```json
{
  "dirName": "brainstorming",
  "name": "brainstorming",
  "description": "Explore ideas into designs and specs",
  "path": "C:/Users/zxxk/.skills-library/brainstorming",
  "valid": true,
  "validationErrors": []
}
```

无效 skill 的示例：

```json
{
  "dirName": "old-skill",
  "path": "C:/Users/zxxk/.skills-library/old-skill",
  "valid": false,
  "validationErrors": [
    "Missing SKILL.md",
    "Missing frontmatter.description"
  ]
}
```

---

## 6. 核心流程

### 6.1 安装 skill 到目标

流程：

1. 用户在目标上下文勾选 skill。
2. 系统检查 skill 是否有效。
3. 系统检查目标目录是否存在且可写。
4. 系统检查目标安装位置是否已有同名内容。
5. 如果已有内容但不是本软件记录的链接，则阻止安装并提示冲突。
6. 如果检查通过，则按平台策略创建 junction 或 symlink。
7. 成功后写入安装记录。
8. UI 刷新状态为已安装。

### 6.2 从目标卸载 skill

流程：

1. 用户取消勾选已安装 skill。
2. 系统根据安装记录找到对应 linkPath。
3. 系统再次验证当前路径确实是这条记录对应的链接。
4. 只有当它是本软件记录过的链接时才删除。
5. 删除成功后移除安装记录。
6. 如果删除失败，保留记录并显示异常状态。

### 6.3 从主目录删除 skill

流程：

1. 用户点击删除主目录 skill。
2. 系统显示二次确认。
3. 系统查找所有指向该 skill 的安装记录。
4. 逐个清理这些已记录链接。
5. 如果任一链接清理失败，则中止主目录删除。
6. 如果全部成功，则直接删除主目录中的真实 skill。
7. 移除相关安装记录。

### 6.4 冲突与异常

- **冲突**：目标已有同名真实目录、普通文件或未知链接，禁止安装。
- **缺失**：记录存在但链接已被手工删除，标记为缺失，不自动重建。
- **异常**：记录与当前链接目标不一致，标记为异常，不自动修复。
- **源缺失**：主目录 skill 被手工删除，相关目标显示源缺失。

---

## 7. 错误处理原则

- 任何删除动作都必须先验证是本软件创建并记录过的对象。
- 未记录的真实目录、未知链接、普通文件一律不自动删除。
- 安装失败时不写入安装记录。
- 卸载失败时不删除安装记录。
- 主目录删除 skill 时，必须在链接清理全部成功后才删除真实 skill。
- 主目录删除不提供恢复能力，UI 必须明确提示不可撤销。

---

## 8. 测试与验收

### 8.1 Skill 校验

验收要点：

- 有 `SKILL.md` 且 frontmatter 含 `name` / `description` 的目录可安装。
- 缺少 `SKILL.md` 或缺少必要 frontmatter 的目录不可安装。
- 无效 skill 的错误原因可见。

### 8.2 目标管理

验收要点：

- 可新增、编辑、删除目标配置。
- 目标路径不存在或不可写时不能安装。
- 删除目标配置不删除目标目录本身。

### 8.3 安装与卸载

验收要点：

- 安装后目标目录可正常读取主目录 skill 内容。
- 主目录变更后，所有已安装目标立即反映变化。
- 卸载只删除本软件创建的链接。
- 一个目标上的卸载不会影响其他目标。

### 8.4 冲突处理

验收要点：

- 同名真实目录、文件、未知链接都会阻止安装。
- 不覆盖、不移动、不删除冲突内容。

### 8.5 主目录删除

验收要点：

- 删除前有明显确认。
- 会先清理所有已记录链接。
- 任一链接清理失败时，不删除真实 skill。
- 删除成功后记录同步移除。

---

## 9. 依赖与实现约束

- 本期不绑定具体桌面技术栈。
- UI、业务逻辑、文件系统能力必须分层，避免直接耦合。
- 数据结构需要保留 `version` 字段，便于后续升级。
- 目标目录与主目录都必须显式配置，不能通过扫描自动发现。

---

## 10. 后续可扩展方向

本期不实现，但架构上可以预留：

- 多主目录支持。
- 批量安装预览。
- 安装历史审计。
- 恢复 / 回收站。
- 搜索、标签、分类。
- 目标模板或 profile。

---

## 11. 结论

第一版建议采用“**单一主目录 + 多目标目录 + 即时链接安装/卸载 + 严格安全删除**”的方案。它与当前需求最一致，界面简单，行为清晰，且能把风险控制在只操作软件自己创建并记录过的链接范围内。
