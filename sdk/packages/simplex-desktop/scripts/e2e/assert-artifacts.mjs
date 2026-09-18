import { existsSync, readFileSync } from "node:fs"
import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
const packagesRoot = resolve(desktopRoot, "..")
const trayStates = ["starting", "setup", "running", "paused", "stopping", "stopped", "unreachable"]
const requiredArtifacts = [
	resolve(packagesRoot, "sdk/dist/node/index.js"),
	resolve(packagesRoot, "simplex/dist/bin/simplex.js"),
	resolve(packagesRoot, "simplex/dist/ui/index.html"),
	resolve(packagesRoot, "simplex/dist/ui/icons/mobile-logo.svg"),
	resolve(packagesRoot, "simplex/src/proto/mpcvault/platform/v1/api.ts"),
	resolve(desktopRoot, "dist/main.js"),
	resolve(desktopRoot, "resources/tray/app.png"),
	...trayStates.flatMap((state) => [
		resolve(desktopRoot, "resources/tray", `${state}.png`),
		resolve(desktopRoot, "resources/tray", `${state}Template.png`),
		resolve(desktopRoot, "resources/tray", `${state}Template@2x.png`),
	]),
]

const missing = requiredArtifacts.filter((path) => !existsSync(path))
if (missing.length > 0) {
	throw new Error(`Simplex desktop E2E artifacts are missing:\n${missing.map((path) => `- ${path}`).join("\n")}`)
}

const desktopMain = readFileSync(resolve(desktopRoot, "dist/main.js"), "utf8")
if (/from ["']electron-updater["']|require\(["']electron-updater["']\)/.test(desktopMain)) {
	throw new Error("The packaged Electron main process must bundle electron-updater")
}

function pngDimensions(path) {
	const png = readFileSync(path)
	return { width: png.readUInt32BE(16), height: png.readUInt32BE(20) }
}

for (const state of trayStates) {
	const standard = resolve(desktopRoot, "resources/tray", `${state}Template.png`)
	const retina = resolve(desktopRoot, "resources/tray", `${state}Template@2x.png`)
	if (JSON.stringify(pngDimensions(standard)) !== JSON.stringify({ width: 18, height: 18 })) {
		throw new Error(`${standard} must be an 18x18 macOS template image`)
	}
	if (JSON.stringify(pngDimensions(retina)) !== JSON.stringify({ width: 36, height: 36 })) {
		throw new Error(`${retina} must be a 36x36 macOS Retina template image`)
	}
}
