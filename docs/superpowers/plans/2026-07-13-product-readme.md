# Product README Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the technical README with a Chinese product landing page per `docs/superpowers/specs/2026-07-13-product-readme-design.md`.

**Architecture:** Root `README.md` becomes the Chinese marketing page (GitHub default). `README.zh.md` redirects to `README.md` to avoid drift. Add a short `README.en.md` English teaser that links to the Chinese README. No app/code changes.

**Tech Stack:** Markdown only.

**Spec:** `docs/superpowers/specs/2026-07-13-product-readme-design.md`

---

## File Map

| File | Responsibility |
|------|----------------|
| `README.md` | Chinese product landing page (default GitHub view) |
| `README.zh.md` | One-line redirect to `README.md` |
| `README.en.md` | Short English teaser + link to Chinese README |

---

### Task 1: Write Chinese product README.md

**Files:**
- Modify: `README.md` (full replace)

- [ ] **Step 1: Replace `README.md` with the following content**

```markdown
# Skills Sync Manager

给团队的 Skills 一份可信来源——统一维护，安全同步到 Cursor、Claude Code、Codex 等多个 Agent，告别复制粘贴和版本混乱。

**[下载安装](https://github.com/xiaoai-lazy/skills-sync-manager/releases)** · [5 分钟上手](#5-分钟上手) · [English](README.en.md)

---

## 团队 Skills 为何总是乱

一份「接口文档规范」Skill，先在某人的 Cursor 里跑通；很快同事要在 Claude Code 里用同一套规则，项目组又要在 Codex 里复用。Skill 从个人笔记，变成团队规范——麻烦也从这里开始：

1. **版本对不齐**：有人已经更新，有人还在旧版；同名 Skill 多份，不知道哪份算数。
2. **同步靠人肉**：改完要复制、要提醒；漏一个人，下次生成就回到旧规则。
3. **状态看不见**：哪些已装到哪个 Agent 或项目、哪些失效，只能翻目录猜。
4. **共享缺入口**：公司内部 Skills 散落在个人电脑、群文件、临时仓库，新人找不到「官方版」。

问题不在 Agent 太多，而在 Skill 没有统一入口。

---

## 终结复制粘贴：从团队仓库直达每个 Agent

1. **团队 Skills 仓库（可信来源）**  
   把团队共用的 Skills 放在 **自建 GitLab** 或 **私有 Skill 中心**。大家从同一入口发现、安装、更新，不再靠群文件和个人拷贝。

2. **本机主库（分发中枢）**  
   你电脑上的主库，用来收拢团队 Skills（以及个人本地 Skills），再分发到本机各级目录与各个 Agent。  
   它是分发中枢，不是团队仓库本身。

3. **安全同步到 Agent / 项目**  
   在侧边栏添加 Cursor、Claude Code、Codex 等目标，勾选需要的 Skill，一键以安全链接装过去。改本机主库里的那一份，各处跟着一致；不覆盖目标里已有的真实文件。

GitLab / 私有 Skill 中心负责「团队哪份算数」；本机主库负责「装到我的各个 Agent」；同步负责「真正用上且不踩坑」。

---

## 你能得到什么

- **接入团队仓库**：连接自建 GitLab 或私有 Skill 中心，从统一入口浏览、安装、更新 Skills
- **本机主库分发**：把要用的 Skills 收拢到本机主库，再同步到多个 Agent / 项目目录
- **一键启用与卸载**：按目标勾选 Skill，自动以安全链接部署；不需要的随时卸掉
- **状态一目了然**：哪些有效、哪些异常待修，不用翻目录猜
- **粘贴即装**：把 GitHub 链接贴进来，预览后装进本机主库
- **密钥就地管理**：私有 GitLab 按站点保存访问密钥，可查看、更新、移除
- **启动按需刷新**：可按来源开关启动时静默刷新，团队仓库有更新更容易看见
- **用着安心**：不扫全盘、不覆盖目标里已有真实文件；危险操作二次确认
- **应用内升级**：有新版本时可在应用内更新

---

## 下载安装

前往 [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases) 下载对应系统的安装包：

- **Windows**：`.exe` / `.msi`
- **macOS**：`.dmg`
- **Linux**：`.AppImage` / `.deb`

若系统提示「不明来源」或 SmartScreen 警告，按系统指引允许打开即可。

---

## 5 分钟上手

1. 安装并打开 Skills Sync Manager，进入 **Skill 中心**，设置本机 **主库** 目录。
2. 打开 **来源管理**，接入团队仓库（自建 GitLab 或私有 Skill 中心）。私有 GitLab 会在需要时提示配置访问密钥，密钥保存在系统凭证库。
3. 在可安装列表中浏览，将需要的 Skills 安装到本机主库；也可粘贴 GitHub 链接快速安装。
4. 在侧边栏 **Agent** 或 **项目** 下添加目标目录（可用预设一键添加）。
5. 选中目标，勾选需要同步的 Skills，完成部署。

---

## 用着安心

- 只操作你指定的目录，不扫描全盘
- 绝不覆盖目标里已有的真实文件或文件夹
- 只卸载本工具创建的链接，不动你原来的内容
- 无效 Skill 不会被装上
- 删除主库 Skill 需二次确认（删除后不可恢复）

---

## 相关链接

- [下载最新版本](https://github.com/xiaoai-lazy/skills-sync-manager/releases)
- [私有 Skill 中心服务端](https://github.com/xiaoai-lazy/skill-hub-server)（可选自建）
```

- [ ] **Step 2: Skim for forbidden terms**

Confirm the file does **not** contain: `storageKey`, `linkStrategy`, `runtime-cache`, `cosign`, `Sigstore`, `install_hub_skill`, `PAT` as a section title (inline「访问密钥」OK), `Skill Hub` (must be「私有 Skill 中心」).

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: rewrite README as Chinese product landing page"
```

---

### Task 2: Redirect README.zh.md and add README.en.md

**Files:**
- Modify: `README.zh.md` (full replace)
- Create: `README.en.md`

- [ ] **Step 1: Replace `README.zh.md` with**

```markdown
# Skills Sync Manager

中文说明已合并到仓库根目录 [README.md](README.md)（GitHub 默认展示）。

English teaser: [README.en.md](README.en.md)
```

- [ ] **Step 2: Create `README.en.md` with**

```markdown
# Skills Sync Manager

Give your team one trusted source for Skills — maintain them in a shared GitLab repo or private Skill center, sync safely to Cursor, Claude Code, Codex, and more. No more copy-paste chaos.

**Full product guide (Chinese):** [README.md](README.md)

**Download:** [GitHub Releases](https://github.com/xiaoai-lazy/skills-sync-manager/releases)

### In one minute

1. Set a **local main library** on your machine (distribution hub).
2. Connect your **team Skills repo** (self-hosted GitLab or private Skill center).
3. Install Skills into the local library, then sync them to Agent / project targets.

Local main library ≠ team repo. The team repo is the source of truth for the org; the local library is how you distribute to your Agents.
```

- [ ] **Step 3: Commit**

```bash
git add README.zh.md README.en.md
git commit -m "docs: point zh README to root and add English teaser"
```

---

### Task 3: Final check

- [ ] **Step 1: Verify structure**

Open `README.md` and confirm section order:

1. Title + subtitle + CTAs  
2. 团队 Skills 为何总是乱  
3. 终结复制粘贴：从团队仓库直达每个 Agent  
4. 你能得到什么  
5. 下载安装  
6. 5 分钟上手  
7. 用着安心  
8. 相关链接  

- [ ] **Step 2: Confirm product model wording**

「本机主库」= 分发中枢；「团队仓库」= GitLab / 私有 Skill 中心。二者不得混为一谈。

---

## Spec coverage

| Spec item | Task |
|-----------|------|
| Chinese landing in `README.md` | Task 1 |
| Title B for solution section | Task 1 |
| Product model (team repo vs local library) | Task 1 |
| 私有 Skill 中心 wording | Task 1 |
| No Cosign | Task 1 Step 2 |
| No developer/migration sections | Task 1 |
| `README.zh.md` / English teaser | Task 2 |
