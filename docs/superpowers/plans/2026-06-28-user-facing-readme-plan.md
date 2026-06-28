# User-Facing README Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `README.md` and `README.zh.md` as user-facing product and installation guides that explain the project goal, the problem it solves, and how to install it.

**Architecture:** This is a documentation-only change. The English and Chinese READMEs will share the same section order and user-facing content model, with development details moved to a short final section. No application code, release workflow, package version, or tests are changed.

**Tech Stack:** Markdown documentation, GitHub Releases links, existing Tauri/React/Rust project context.

---

## File Structure

- Modify: `README.md`
  - Responsibility: Primary English user-facing README.
  - Replace developer-first structure with product-first structure.
- Modify: `README.zh.md`
  - Responsibility: Chinese user-facing README with matching structure and equivalent content.
- Reference only: `docs/superpowers/specs/2026-06-28-user-facing-readme-design.md`
  - Responsibility: Approved README design spec.
- Do not modify: application source, tests, package/version files, release workflow.

---

### Task 1: Rewrite English README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace `README.md` with the approved user-facing structure**

Write this exact content to `README.md`:

```markdown
# Skills Sync Manager

[中文文档](README.zh.md)

Skills Sync Manager is a desktop app for keeping one main skills library in sync with multiple Claude / agent target directories.

Use it when you want one source of truth for your skills, but need those skills available in several local tool directories without copying files by hand.

## Why this exists

Managing skills across multiple agent directories gets messy quickly:

- Copying skills by hand creates stale duplicates.
- Manual symlinks or junctions are hard to audit later.
- Deleting a skill can leave broken links behind.
- Overwriting an existing target directory can destroy work.

Skills Sync Manager gives you a safer workflow: keep your skills in one main directory, add the target directories you use, and choose which skills should be linked into each target.

## What you can do

- Set one main skills directory as your source library.
- Add multiple target directories for Claude or other local agents.
- See which skills are valid and which ones need fixing.
- Install or uninstall selected skills for the current target.
- Delete a skill from the main library after explicit confirmation.
- Keep settings, targets, and installation records saved locally.

## Download and install

Pre-built installers are available on the [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) page.

Download the package for your platform:

- **Windows**: `.msi` or `.exe`
- **macOS**: `.dmg`
- **Linux**: `.AppImage` or `.deb`

> The installers are currently unsigned. Windows may show a SmartScreen warning, and macOS may require right-click → Open. This is a code-signing status note; signing will be added in a future release.

## First run

1. Open Skills Sync Manager.
2. Set your main skills directory.
3. Add one or more target directories.
4. Select a target directory from the sidebar.
5. Turn on the skills you want available in that target.

Each skill should be a direct child directory of the main skills directory. A valid skill contains a `SKILL.md` file with YAML frontmatter fields for `name` and `description`.

## Safety model

Skills Sync Manager is intentionally conservative:

- It does not scan your machine for agent directories.
- It does not overwrite existing files or real directories in a target.
- It only uninstalls links that it created and recorded.
- It shows invalid skills but prevents installing them.
- Deleting a main-library skill is irreversible and requires confirmation.

## Link behavior

- **Windows**: uses junctions by default.
- **macOS / Linux**: uses directory symlinks by default.

## Developer notes

Tech stack:

- Tauri 2
- React
- TypeScript
- Vite
- Rust

Run locally:

```bash
npm install
npm run tauri:dev
```

Verify changes:

```bash
npm run test
npm run build
cd src-tauri && cargo test
```

## Manual testing

See [docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md](docs/tasks/task-20260623-skills-sync-manager/skills-sync-manager-test.md) for the cross-platform verification checklist.
```

- [ ] **Step 2: Review English README content**

Check that `README.md`:

- Starts with project value, not technology.
- Includes the GitHub Releases installation link.
- Keeps unsigned installer warning.
- Keeps developer commands only near the end.
- Does not mention manual release tagging instructions.

Run:

```bash
git diff -- README.md
```

Expected: only `README.md` changed, with the section order shown above.

---

### Task 2: Rewrite Chinese README

**Files:**
- Modify: `README.zh.md`

- [ ] **Step 1: Replace `README.zh.md` with matching Chinese structure**

Write this exact content to `README.zh.md`:

```markdown
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

> 当前安装包尚未签名。Windows 可能会显示 SmartScreen 警告，macOS 可能需要右键 → 打开。这是代码签名状态说明；后续版本会补充签名。

## 首次使用

1. 打开 Skills Sync Manager。
2. 设置主 skills 目录。
3. 添加一个或多个目标目录。
4. 从侧边栏选择一个目标目录。
5. 打开你希望同步到该目标目录的 skills。

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
```

- [ ] **Step 2: Review Chinese README content**

Check that `README.zh.md`:

- Matches the English README section order.
- Uses user-facing Chinese copy, not developer-first wording.
- Includes the GitHub Releases installation link.
- Keeps unsigned installer warning.
- Keeps developer commands only near the end.

Run:

```bash
git diff -- README.zh.md
```

Expected: only `README.zh.md` changed, with the matching Chinese structure shown above.

---

### Task 3: Verify Documentation Rewrite

**Files:**
- Verify: `README.md`
- Verify: `README.zh.md`

- [ ] **Step 1: Check markdown links and section consistency**

Run:

```bash
python - <<'PY'
from pathlib import Path
checks = {
    'README.md': [
        '# Skills Sync Manager',
        '[中文文档](README.zh.md)',
        '## Why this exists',
        '## What you can do',
        '## Download and install',
        '## First run',
        '## Safety model',
        '## Link behavior',
        '## Developer notes',
        '## Manual testing',
        'https://github.com/xiaoai-lazy/skills-sync-manager/releases',
    ],
    'README.zh.md': [
        '# Skills Sync Manager',
        '[English README](README.md)',
        '## 为什么需要它',
        '## 你可以做什么',
        '## 下载与安装',
        '## 首次使用',
        '## 安全边界',
        '## 链接行为',
        '## 开发者信息',
        '## 手动测试',
        'https://github.com/xiaoai-lazy/skills-sync-manager/releases',
    ],
}
for file, required in checks.items():
    text = Path(file).read_text(encoding='utf-8')
    missing = [item for item in required if item not in text]
    if missing:
        raise SystemExit(f'{file} missing: {missing}')
print('README checks passed')
PY
```

Expected output:

```text
README checks passed
```

- [ ] **Step 2: Confirm no application files changed**

Run:

```bash
git status --short
```

Expected: only these files are modified:

```text
 M README.md
 M README.zh.md
```

If previously unrelated local changes exist, do not include them in this README commit.

- [ ] **Step 3: Commit README rewrite**

Run:

```bash
git add README.md README.zh.md
git commit -m "docs: make readme user-facing"
```

Expected: commit succeeds with only README changes.

---

## Self-Review

- Spec coverage: The plan covers user-facing positioning, problem statement, install instructions, first-run flow, safety boundaries, link behavior, developer notes, and manual testing for both English and Chinese READMEs.
- Placeholder scan: No TBD/TODO/placeholder language remains.
- Scope check: This is documentation-only and does not modify application code, tests, release workflow, or versions.
- Consistency check: English and Chinese READMEs use matching section order and equivalent content.
