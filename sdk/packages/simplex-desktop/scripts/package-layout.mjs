import { readFileSync } from "node:fs"
import { access, chmod, copyFile, mkdir } from "node:fs/promises"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { SIMPLEX_BUNDLE_EXTERNALS } from "../../simplex/bundle-externals.mjs"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const runtimeManifest = JSON.parse(readFileSync(join(packageRoot, "runtime", "package.json"), "utf8"))
const runtimePackages = Object.keys(runtimeManifest.dependencies ?? {}).sort()
const bundlePackages = [...SIMPLEX_BUNDLE_EXTERNALS].sort()
if (JSON.stringify(runtimePackages) !== JSON.stringify(bundlePackages)) {
	throw new Error("The packaged runtime dependencies do not match the Simplex bundle externals")
}
export const EXTERNAL_RUNTIME_PACKAGES = bundlePackages

export function runtimeTarget(platform, arch) {
	if (platform === "darwin" && ["arm64", "x64", "universal"].includes(arch)) return `darwin-${arch}`
	if (platform === "linux" && ["arm64", "x64"].includes(arch)) return `linux-${arch}`
	if (platform === "win32" && arch === "x64") return "win32-x64"
	throw new Error(`No packaged Simplex runtime for ${platform}-${arch}`)
}

export function packagedResourcesDirectory({ appOutDir, platform, productFilename }) {
	return platform === "darwin"
		? join(appOutDir, `${productFilename}.app`, "Contents", "Resources")
		: join(appOutDir, "resources")
}

export async function assertPackagedResources(resourcesDirectory, platform) {
	const executable = platform === "win32" ? "node.exe" : "node"
	const required = [
		join(resourcesDirectory, "runtime", executable),
		join(resourcesDirectory, "simplex", "package.json"),
		join(resourcesDirectory, "simplex", "dist", "bin", "simplex.js"),
		join(resourcesDirectory, "simplex", "dist", "ui", "index.html"),
		...EXTERNAL_RUNTIME_PACKAGES.map((name) => join(resourcesDirectory, "node_modules", name, "package.json")),
	]
	for (const path of required) await access(path)
}

export async function installPackagedRuntime({ appOutDir, platform, arch, productFilename }) {
	const target = runtimeTarget(platform, arch)
	const executable = platform === "win32" ? "node.exe" : "node"
	const source = join(packageRoot, "resources", "node", target, executable)
	const resourcesDirectory = packagedResourcesDirectory({ appOutDir, platform, productFilename })
	const destination = join(resourcesDirectory, "runtime", executable)
	await mkdir(dirname(destination), { recursive: true })
	await copyFile(source, destination)
	if (platform !== "win32") await chmod(destination, 0o755)
	await assertPackagedResources(resourcesDirectory, platform)
	return destination
}
