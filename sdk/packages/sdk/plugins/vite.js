import { existsSync } from "node:fs"
import { copyFile } from "node:fs/promises"
import path from "node:path"
import { colorize } from "consola/utils"
import { findMonorepoRootHeuristic } from "./monorepo-helper.js";

/** eg. silent | debug | default */
let debug_mode = "default";
const validLevels = new Set(['debug', 'default', 'silent'])

const logMessage = (message) => {
  if ('silent' === debug_mode) return;

  const time = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: true,
  })

  const timestamp = colorize("dim", time)
  const tag = colorize("bold", colorize("magenta", "[hyperbridge]"))

  return console.log(timestamp + tag, message)
}

const logDebug = (message) => {
  if (debug_mode !== 'debug') return;

  const time = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: true,
  })

  const timestamp = colorize("dim", time)
  const tag = colorize("bold", colorize("magenta", "[hyperbridge]"))
  const debug = colorize("cyan", "[debug]");

  return console.debug(timestamp, tag, debug, message)
}

const findWasmSource = (projectDir) => {
  const projectNodeModules = path.resolve(projectDir, "node_modules");
  let source = path.resolve(projectNodeModules, "@hyperbridge/sdk/dist/browser/web_bg.wasm");

  if (existsSync(source)) return { source, tag: "Project Root" }

  logDebug("Could not find wasm dependency in project node_modules, checking monorepo root...");
  const monorepoRoot = findMonorepoRootHeuristic(projectDir);

  if (!monorepoRoot) return null;

  const monorepoNodeModules = path.resolve(monorepoRoot, "node_modules");
  source = path.resolve(monorepoNodeModules, "@hyperbridge/sdk/dist/browser/web_bg.wasm");

  if (!existsSync(source)) return null;

  logDebug(`Found wasm dependency in monorepo root: ${source}`);
  return { source, tag: "Monorepo Root" }
}

const waitAndCopy = async ({ source, dest, destDir }) => {
  const interval = 2000; // 2 seconds
  const timeout = 60000; // 60 seconds
  let elapsedTime = 0;

  while (elapsedTime < timeout) {
    if (existsSync(destDir)) {
      logMessage(`📦 Copying wasm dependency from ${source.tag}`);
      try {
        await copyFile(source.source, dest);
        logMessage("✅ Copy complete");
        return;
      } catch (error) {
        logMessage(`❌ Error copying wasm file: ${error?.message}`);
        return;
      }
    } else {
      logMessage(`... waiting for ${destDir} to be created (retrying in 2s)`);
      await new Promise(resolve => setTimeout(resolve, interval));
      elapsedTime += interval;
    }
  }
  logMessage(`❌ Timed out waiting for ${destDir} to be created.`);
}

/**
 *
 * @returns {import('vite').PluginContainer}
 */
const copyWasm = (params = {}) => {
  const { logLevel = "default" } = params;

  if (!validLevels.has(logLevel)) {
    throw new Error(`Invalid log level: ${logLevel}. Should be one of: ${Array.from(validLevels.values()).join(", ")}`);
  }

  debug_mode = logLevel;

  let is_dev_server = false;


  return {
    name: "@hyperbridge/vite:wasm-deps",
    configResolved(config) {
      if (config.command === "serve") {
        is_dev_server = true
      }
    },
    buildStart: function makeCopy() {
      if (!is_dev_server) {
        logMessage("⏭️ Skipping wasm dependency. Not neccessary for bundling step");
        return;
      }

      const source = findWasmSource(process.cwd());

      if (!source) {
        logMessage("❌ Could not find wasm dependency.");
        return;
      }

      const projectNodeModules = path.resolve(process.cwd(), "node_modules");
      const destDir = path.resolve(projectNodeModules, "./.vite/deps");
      const dest = path.resolve(destDir, "web_bg.wasm");

      waitAndCopy({ source, dest, destDir });
    }
  }
}

export default copyWasm
