import { copyFile } from "node:fs/promises"
import path from "node:path"
import { colorize } from "consola/utils"

const logMessage = (message) => {
  const time = new Date().toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: true,
  })

  const timestamp = colorize("dim", time)
  const tag = colorize("bold", colorize("magenta", "[hyperbridge]"))

  return console.log(timestamp, tag, message)
}

/**
 *
 * @returns {Plugin}
 */
const copyWasm = () => {
  let is_dev_server = false

  return {
    name: "@hyperbridge/vite:wasm-deps",
    configResolved(config) {
      if (config.command === "serve") {
        is_dev_server = true
      }
    },
    buildStart: async function makeCopy() {
      if (!is_dev_server) {
        logMessage("⏭️ Skipping wasm dependency. No neccessary in build step");
        return;
      }

      // @todo: Add monorepo support

      // Get path to the consuming project's node_modules
      const projectNodeModules = path.resolve(process.cwd(), "node_modules")

      // Find the @hyperbridge/sdk package in node_modules
      const source = path.resolve(
        projectNodeModules,
        "@hyperbridge/sdk/dist/browser/web_bg.wasm",
      )

      // Destination in the Vite cache directory
      const destDir = path.resolve(projectNodeModules, ".vite/deps")
      const dest = path.resolve(destDir, "web_bg.wasm")

      // Wait for .vite folder to exist
      setTimeout(async () => {
        try {
          logMessage("📦 Copying wasm dependency")
          await copyFile(source, dest)
          logMessage("✅ Copy complete")
        } catch (error) {
          logMessage(`❌ Error copying wasm file: ${error?.message}`)
        }
      }, 2000)
    },
  }
}

export default copyWasm
