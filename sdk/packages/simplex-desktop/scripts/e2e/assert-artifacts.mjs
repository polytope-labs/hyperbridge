import { existsSync } from "node:fs"
import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
const packagesRoot = resolve(desktopRoot, "..")
const requiredArtifacts = [
	resolve(packagesRoot, "sdk/dist/node/index.js"),
	resolve(packagesRoot, "simplex/dist/bin/simplex.js"),
	resolve(packagesRoot, "simplex/dist/ui/index.html"),
	resolve(packagesRoot, "simplex/src/proto/mpcvault/platform/v1/api.ts"),
	resolve(desktopRoot, "dist/main.js"),
]

const missing = requiredArtifacts.filter((path) => !existsSync(path))
if (missing.length > 0) {
	throw new Error(`Simplex desktop E2E artifacts are missing:\n${missing.map((path) => `- ${path}`).join("\n")}`)
}
