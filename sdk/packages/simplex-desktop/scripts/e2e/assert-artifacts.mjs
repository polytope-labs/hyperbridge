import { existsSync, readFileSync } from "node:fs"
import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { PNG } from "pngjs"

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
const packagesRoot = resolve(desktopRoot, "..")
const trayStates = ["starting", "setup", "running", "paused", "stopping", "stopped", "unreachable"]
const applicationIconSizes = [16, 32, 48, 64, 128, 256, 512]
const requiredArtifacts = [
	resolve(packagesRoot, "sdk/dist/node/index.js"),
	resolve(packagesRoot, "simplex/dist/bin/simplex.js"),
	resolve(packagesRoot, "simplex/dist/ui/index.html"),
	resolve(packagesRoot, "simplex/dist/ui/icons/mobile-logo.svg"),
	resolve(packagesRoot, "simplex/src/proto/mpcvault/platform/v1/api.ts"),
	resolve(desktopRoot, "dist/main.js"),
	resolve(desktopRoot, "THIRD-PARTY-NOTICES.md"),
	resolve(desktopRoot, "node_modules/electron/dist/LICENSE"),
	resolve(desktopRoot, "node_modules/electron/dist/LICENSES.chromium.html"),
	resolve(desktopRoot, "resources/icon.icns"),
	resolve(desktopRoot, "resources/icon.ico"),
	resolve(desktopRoot, "resources/tray/app.png"),
	...applicationIconSizes.map((size) => resolve(desktopRoot, "resources/icons", `${size}x${size}.png`)),
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

function chromaticPixelRatio(path, { x, y, width, height }) {
	const png = PNG.sync.read(readFileSync(path))
	let chromaticPixels = 0

	for (let row = y; row < y + height; row += 1) {
		for (let column = x; column < x + width; column += 1) {
			const offset = (png.width * row + column) * 4
			const red = png.data[offset]
			const green = png.data[offset + 1]
			const blue = png.data[offset + 2]
			if (Math.max(red, green, blue) - Math.min(red, green, blue) >= 32) {
				chromaticPixels += 1
			}
		}
	}

	return chromaticPixels / (width * height)
}

for (const size of applicationIconSizes) {
	const icon = resolve(desktopRoot, "resources/icons", `${size}x${size}.png`)
	if (JSON.stringify(pngDimensions(icon)) !== JSON.stringify({ width: size, height: size })) {
		throw new Error(`${icon} must be a ${size}x${size} application icon`)
	}
}
const applicationIcon = resolve(desktopRoot, "resources/tray/app.png")
if (JSON.stringify(pngDimensions(applicationIcon)) !== JSON.stringify({ width: 512, height: 512 })) {
	throw new Error(`${applicationIcon} must be a 512x512 application icon`)
}
for (const icon of [applicationIcon, resolve(desktopRoot, "resources/icons/512x512.png")]) {
	const centerChromaticRatio = chromaticPixelRatio(icon, { x: 144, y: 144, width: 224, height: 224 })
	if (centerChromaticRatio < 0.25) {
		throw new Error(
			`${icon} must preserve the PWA logo's multicolored center (chromatic pixel ratio: ${centerChromaticRatio.toFixed(3)})`,
		)
	}
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
