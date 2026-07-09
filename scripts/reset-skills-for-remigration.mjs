/**
 * Backup → remove agent junction links → clear skill library + config records.
 * Keeps: version, settings, targets, projects, skillRepos, skillHubEndpoints.
 *
 * Usage (close Cursor/Claude/Codex first):
 *   node scripts/reset-skills-for-remigration.mjs
 *   node scripts/reset-skills-for-remigration.mjs --dry-run
 */

import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const dryRun = process.argv.includes("--dry-run");
const stamp = new Date().toISOString().replace(/[-:]/g, "").replace(/\..+/, "").replace("T", "-");
const configPath = join(
  homedir(),
  "AppData/Roaming/com.xiaoai-lazy.skills-sync-manager/config.json",
);
const backupRoot = join("C:", `skills-reset-backup-${stamp}`);
const mainSkillsDir = "C:\\skills";

function log(msg) {
  console.log(dryRun ? `[dry-run] ${msg}` : msg);
}

function readConfig() {
  return JSON.parse(readFileSync(configPath, "utf8"));
}

function isReparsePoint(path) {
  try {
    const st = statSync(path);
    return (st.mode & 0o170000) === 0o120000 || st.isDirectory() && hasJunctionAttribute(path);
  } catch {
    return false;
  }
}

function hasJunctionAttribute(path) {
  if (process.platform !== "win32") return false;
  try {
    const item = readdirSync(path);
    return false;
  } catch {
    return false;
  }
}

function removeJunction(linkPath) {
  if (!existsSync(linkPath)) {
    log(`skip missing link: ${linkPath}`);
    return "missing";
  }
  log(`remove junction: ${linkPath}`);
  if (dryRun) return "dry";
  try {
    execSync(`cmd /c rmdir "${linkPath}"`, { stdio: "pipe" });
    return "removed";
  } catch (err) {
    console.error(`FAILED to remove ${linkPath}: ${err.stderr?.toString() || err.message}`);
    return "failed";
  }
}

function collectTargetSkillDirs(config) {
  const dirs = new Set();
  for (const target of config.targets ?? []) {
    if (target.skillsDir) dirs.add(target.skillsDir);
  }
  return [...dirs];
}

function scanJunctions(skillDirs) {
  const found = [];
  for (const dir of skillDirs) {
    if (!existsSync(dir)) continue;
    for (const name of readdirSync(dir)) {
      const full = join(dir, name);
      try {
        const st = statSync(full);
        if (st.isDirectory()) {
          found.push(full);
        }
      } catch {
        /* ignore */
      }
    }
  }
  return found;
}

function backup() {
  log(`backup root: ${backupRoot}`);
  if (dryRun) return;
  mkdirSync(backupRoot, { recursive: true });
  cpSync(configPath, join(backupRoot, "config.json"));
  if (existsSync(mainSkillsDir)) {
    cpSync(mainSkillsDir, join(backupRoot, "skills"), { recursive: true });
  }
  writeFileSync(
    join(backupRoot, "README.txt"),
    [
      "Skills Manager reset backup",
      `Created: ${new Date().toISOString()}`,
      "",
      "Restore config:",
      `  copy config.json -> ${configPath}`,
      "",
      "Restore skills library:",
      `  xcopy /E /I skills ${mainSkillsDir}`,
    ].join("\n"),
    "utf8",
  );
}

function clearMainLibrary() {
  for (const sub of ["repo", "local", "hub"]) {
    const p = join(mainSkillsDir, sub);
    if (!existsSync(p)) continue;
    log(`clear ${p}`);
    if (!dryRun) rmSync(p, { recursive: true, force: true });
  }
}

function clearConfigRecords(config) {
  config.installations = [];
  config.skillRecords = {};
  config.skillDiscoverCache = { fetchedAt: "", skills: [] };
  config.skillUpdateCache = { checkedAt: "", updates: [] };
  config.version = 6;
  return config;
}

function main() {
  if (!existsSync(configPath)) {
    console.error(`config not found: ${configPath}`);
    process.exit(1);
  }

  const config = readConfig();
  const mainDir = config.settings?.mainSkillsDir ?? mainSkillsDir;
  log(`config version=${config.version}, mainSkillsDir=${mainDir}`);
  log(`installations=${config.installations?.length ?? 0}, skillRecords=${Object.keys(config.skillRecords ?? {}).length}`);

  backup();

  const linkPaths = new Set([
    ...(config.installations ?? []).map((i) => i.linkPath),
    ...scanJunctions(collectTargetSkillDirs(config)),
  ]);

  const results = { removed: 0, failed: 0, missing: 0, dry: 0 };
  for (const linkPath of linkPaths) {
    const r = removeJunction(linkPath);
    results[r === "removed" ? "removed" : r === "failed" ? "failed" : r === "dry" ? "dry" : "missing"]++;
  }

  clearMainLibrary();
  const next = clearConfigRecords(config);
  log(`write config: clear installations + skillRecords, keep targets/projects/repos`);
  if (!dryRun) {
    writeFileSync(configPath, JSON.stringify(next, null, 2) + "\n", "utf8");
  }

  console.log("\n=== summary ===");
  console.log(`backup: ${backupRoot}`);
  console.log(`junctions: removed=${results.removed} failed=${results.failed} missing=${results.missing}`);
  console.log(`main library: cleared repo/local/hub under ${mainDir}`);
  console.log("config: installations + skillRecords cleared, version stays 6");
  console.log("\nNext: restart Skills Manager → Skill 中心 → re-install skills to targets");

  if (results.failed > 0) {
    console.error("\nSome junctions could not be removed. Close Cursor/Claude/Codex and re-run.");
    process.exit(1);
  }
}

main();
