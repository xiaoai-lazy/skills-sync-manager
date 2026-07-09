/**
 * Restore main skill library + config records + target junction links from reset backup.
 *
 * Usage (close Cursor/Claude/Codex first):
 *   node scripts/restore-skills-from-backup.mjs
 *   node scripts/restore-skills-from-backup.mjs --backup C:\skills-reset-backup-20260708-033003
 *   node scripts/restore-skills-from-backup.mjs --dry-run
 */

import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  lstatSync,
  writeFileSync,
} from "node:fs";
import { execSync } from "node:child_process";
import { homedir } from "node:os";
import { basename, dirname, join } from "node:path";

const dryRun = process.argv.includes("--dry-run");
const backupArgIdx = process.argv.indexOf("--backup");
const defaultBackup = findLatestBackup();
const backupRoot =
  backupArgIdx >= 0 ? process.argv[backupArgIdx + 1] : defaultBackup;

const configPath = join(
  homedir(),
  "AppData/Roaming/com.xiaoai-lazy.skills-sync-manager/config.json",
);
const mainSkillsDir = "C:\\skills";

function findLatestBackup() {
  const entries = readdirSync("C:\\").filter((n) => n.startsWith("skills-reset-backup-"));
  entries.sort();
  if (entries.length === 0) {
    throw new Error("No skills-reset-backup-* folder found on C:\\");
  }
  return join("C:", entries[entries.length - 1]);
}

function log(msg) {
  console.log(dryRun ? `[dry-run] ${msg}` : msg);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function pathEntryExists(linkPath) {
  try {
    lstatSync(linkPath);
    return true;
  } catch {
    return false;
  }
}

function removeLinkIfExists(linkPath) {
  if (!pathEntryExists(linkPath)) return "missing";
  log(`remove existing link: ${linkPath}`);
  if (dryRun) return "dry";
  try {
    execSync(`cmd /c rmdir "${linkPath}"`, { stdio: "pipe" });
    if (!pathEntryExists(linkPath)) return "removed";
  } catch {
    /* fall through */
  }
  try {
    execSync(
      `powershell -NoProfile -Command "Remove-Item -LiteralPath '${linkPath.replace(/'/g, "''")}' -Force -Recurse -ErrorAction Stop"`,
      { stdio: "pipe" },
    );
    if (!pathEntryExists(linkPath)) return "removed";
  } catch (err) {
    console.error(`cannot remove ${linkPath}: ${err.stderr?.toString() || err.message}`);
    return "failed";
  }
  return pathEntryExists(linkPath) ? "failed" : "removed";
}

function createJunction(sourcePath, linkPath) {
  if (!existsSync(sourcePath)) {
    console.error(`source missing, skip junction: ${sourcePath}`);
    return "no-source";
  }
  mkdirSync(dirname(linkPath), { recursive: true });
  const removed = removeLinkIfExists(linkPath);
  if (pathEntryExists(linkPath)) {
    return removed === "failed" ? "blocked" : "blocked";
  }
  log(`create junction: ${linkPath} -> ${sourcePath}`);
  if (dryRun) return "dry";
  try {
    execSync(`cmd /c mklink /J "${linkPath}" "${sourcePath}"`, { stdio: "pipe" });
    return "created";
  } catch (err) {
    console.error(`mklink failed ${linkPath}: ${err.stderr?.toString() || err.message}`);
    return "failed";
  }
}

function copySkillsLibrary() {
  const src = join(backupRoot, "skills");
  if (!existsSync(src)) {
    throw new Error(`backup skills folder missing: ${src}`);
  }
  log(`copy skills: ${src} -> ${mainSkillsDir}`);
  if (!dryRun) {
    mkdirSync(mainSkillsDir, { recursive: true });
    for (const name of readdirSync(src)) {
      const from = join(src, name);
      const to = join(mainSkillsDir, name);
      if (existsSync(to)) rmSync(to, { recursive: true, force: true });
      cpSync(from, to, { recursive: true });
    }
  }
}

function inferLocalSkillRecords(mainDir, existingRecords) {
  const localRoot = join(mainDir, "local");
  if (!existsSync(localRoot)) return {};

  const added = {};
  for (const name of readdirSync(localRoot)) {
    const skillDir = join(localRoot, name);
    if (!statSync(skillDir).isDirectory()) continue;
    if (!existsSync(join(skillDir, "SKILL.md"))) continue;

    const storageKey = `local/${name}`;
    if (existingRecords[storageKey]) continue;

    added[storageKey] = {
      repoHost: "github.com",
      projectPath: "",
      source: "local",
      repoOwner: "",
      repoName: "",
      repoBranch: "",
      directory: name,
      contentHash: "",
      installedAt: new Date().toISOString(),
      storageKey,
      linkName: name,
      repoSlug: "",
      hubEndpointId: "",
      hubSkillGroup: "",
      hubSkillId: "",
    };
    log(`add local skillRecord: ${storageKey}`);
  }
  return added;
}

function normalizeSkillRecords(records) {
  for (const rec of Object.values(records)) {
    if (rec.repoHost === undefined) rec.repoHost = "github.com";
    if (rec.projectPath === undefined) rec.projectPath = "";
    if (rec.repoOwner === undefined) rec.repoOwner = "";
    if (rec.repoName === undefined) rec.repoName = "";
    if (rec.repoBranch === undefined) rec.repoBranch = "";
    if (rec.contentHash === undefined) rec.contentHash = "";
    if (rec.storageKey === undefined) rec.storageKey = "";
    if (rec.linkName === undefined) rec.linkName = "";
    if (rec.repoSlug === undefined) rec.repoSlug = "";
    if (rec.hubEndpointId === undefined) rec.hubEndpointId = "";
    if (rec.hubSkillGroup === undefined) rec.hubSkillGroup = "";
    if (rec.hubSkillId === undefined) rec.hubSkillId = "";
  }
  return records;
}

function restoreConfig(backupConfig) {
  const current = readJson(configPath);
  current.version = 6;
  current.skillRecords = normalizeSkillRecords({ ...backupConfig.skillRecords });
  Object.assign(current.skillRecords, inferLocalSkillRecords(mainSkillsDir, current.skillRecords));
  normalizeSkillRecords(current.skillRecords);
  current.installations = backupConfig.installations ?? [];
  current.skillRepos = backupConfig.skillRepos ?? current.skillRepos ?? [];
  current.skillDiscoverCache = backupConfig.skillDiscoverCache ?? current.skillDiscoverCache;
  current.skillUpdateCache = backupConfig.skillUpdateCache ?? current.skillUpdateCache;

  log(
    `restore config: skillRecords=${Object.keys(current.skillRecords).length}, installations=${current.installations.length}`,
  );
  if (!dryRun) {
    writeFileSync(configPath, JSON.stringify(current, null, 2) + "\n", "utf8");
  }
  return current;
}

function restoreJunctions(installations) {
  const results = { created: 0, failed: 0, blocked: 0, noSource: 0, dry: 0 };
  for (const inst of installations) {
    const r = createJunction(inst.sourcePath, inst.linkPath);
    if (r === "created") results.created++;
    else if (r === "failed") results.failed++;
    else if (r === "blocked") results.blocked++;
    else if (r === "no-source") results.noSource++;
    else if (r === "dry") results.dry++;
  }
  return results;
}

function main() {
  const backupConfigPath = join(backupRoot, "config.json");
  if (!existsSync(backupConfigPath)) {
    console.error(`backup config not found: ${backupConfigPath}`);
    process.exit(1);
  }

  log(`using backup: ${backupRoot}`);
  const backupConfig = readJson(backupConfigPath);

  copySkillsLibrary();
  const config = restoreConfig(backupConfig);
  const junctionResults = restoreJunctions(config.installations);

  console.log("\n=== restore summary ===");
  console.log(`backup: ${backupRoot}`);
  console.log(`skillRecords: ${Object.keys(config.skillRecords).length}`);
  console.log(`installations: ${config.installations.length}`);
  console.log(
    `junctions: created=${junctionResults.created} blocked=${junctionResults.blocked} noSource=${junctionResults.noSource} failed=${junctionResults.failed}`,
  );
  console.log("\nRestart Skills Manager to verify Skill 中心 and target mappings.");

  if (junctionResults.failed > 0 || junctionResults.blocked > 0) {
    console.error("\nSome junctions were not created. Close Cursor/Claude/Codex and re-run.");
    process.exit(1);
  }
}

main();
