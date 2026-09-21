import { spawn } from "node:child_process"
import { access } from "node:fs/promises"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const electronExecutableByPlatform = {
	darwin: "Electron.app/Contents/MacOS/Electron",
	linux: "electron",
	win32: "electron.exe",
}

export function electronRuntimeFiles(platform = process.platform) {
	const executable = electronExecutableByPlatform[platform]
	if (!executable) throw new Error(`Electron has no supported desktop runtime for ${platform}`)
	return [executable, "LICENSE", "LICENSES.chromium.html"]
}

async function missingRuntimeFiles(electronRoot, platform) {
	const missing = []
	for (const path of electronRuntimeFiles(platform)) {
		try {
			await access(join(electronRoot, "dist", path))
		} catch {
			missing.push(path)
		}
	}
	return missing
}

function runElectronInstaller(script) {
	return new Promise((resolveRun, reject) => {
		const child = spawn(process.execPath, [script], { cwd: dirname(script), stdio: "inherit" })
		child.once("error", reject)
		child.once("exit", (code, signal) => {
			if (code === 0) resolveRun()
			else reject(new Error(`Electron runtime installer exited ${code ?? signal}`))
		})
	})
}

export async function ensureElectronRuntime(options = {}) {
	const electronRoot = options.electronRoot ?? resolve(packageRoot, "node_modules/electron")
	const platform = options.platform ?? process.platform
	const runInstaller = options.runInstaller ?? runElectronInstaller
	if ((await missingRuntimeFiles(electronRoot, platform)).length === 0) return false

	await runInstaller(join(electronRoot, "install.js"))
	const missing = await missingRuntimeFiles(electronRoot, platform)
	if (missing.length > 0) {
		throw new Error(
			`Electron runtime installation did not produce:\n${missing.map((path) => `- ${path}`).join("\n")}`,
		)
	}
	return true
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	await ensureElectronRuntime()
}
