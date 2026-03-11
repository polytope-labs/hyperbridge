import fs from "node:fs";
import path from "node:path";

function hasConventionalMonorepoLayout(dir) {
  const candidates = ["packages", "apps"];
  for (const name of candidates) {
    const p = path.join(dir, name);
    if (fs.existsSync(p) && fs.lstatSync(p).isDirectory()) {
      // Check if there are multiple subdirs with package.json
      const subdirs = fs.readdirSync(p).filter(d => {
        const full = path.join(p, d);
        return fs.lstatSync(full).isDirectory() &&
          fs.existsSync(path.join(full, "package.json"));
      });
      if (subdirs.length >= 2) return true;
    }
  }
  return false;
}

function hasWorkspaces(dir) {
  const pkgPath = path.join(dir, "package.json");
  if (!fs.existsSync(pkgPath)) return false;
  try {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
    if (Array.isArray(pkg.workspaces) && pkg.workspaces.length > 0) return true;
    if (pkg.workspaces && Array.isArray(pkg.workspaces.packages) && pkg.workspaces.packages.length > 0) return true;
  } catch { }
  return false;
}

function hasPnpmWorkspace(dir) {
  return fs.existsSync(path.join(dir, "pnpm-workspace.yaml"));
}

function hasLerna(dir) {
  const lernaPath = path.join(dir, "lerna.json");
  if (!fs.existsSync(lernaPath)) return false;
  try {
    const lerna = JSON.parse(fs.readFileSync(lernaPath, "utf8"));
    return Array.isArray(lerna.packages) && lerna.packages.length > 0;
  } catch { return false; }
}

function hasNx(dir) {
  return fs.existsSync(path.join(dir, "nx.json")) ||
    fs.existsSync(path.join(dir, "workspace.json"));
}

function hasTurbo(dir) {
  return fs.existsSync(path.join(dir, "turbo.json"));
}

function hasRush(dir) {
  return fs.existsSync(path.join(dir, "rush.json"));
}

function isMonorepoDir(dir) {
  return hasWorkspaces(dir) ||
    hasPnpmWorkspace(dir) ||
    hasLerna(dir) ||
    hasNx(dir) ||
    hasTurbo(dir) ||
    hasRush(dir) ||
    hasConventionalMonorepoLayout(dir);
}

/**
 * Example it detects: starting at /repo/apps/web/src
 * - finds nx.json at /repo → returns /repo
 * Or starting at /repo/packages/lib/src
 * - no tool configs, but finds multiple package.json under /repo/packages → returns /repo
 */
export function findMonorepoRootHeuristic(startDir = process.cwd()) {
  let dir = path.resolve(startDir);
  const { root } = path.parse(dir);

  while (true) {
    if (isMonorepoDir(dir)) return dir;
    if (dir === root) return null;
    dir = path.dirname(dir);
  }
}
