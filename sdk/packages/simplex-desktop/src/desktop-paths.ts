import { createHash } from "node:crypto"
import { existsSync } from "node:fs"
import { tmpdir } from "node:os"
import { join, posix, win32 } from "node:path"

/** Conservative common denominator below Darwin's 104-byte and Linux's 108-byte sun_path limits. */
export const UNIX_SOCKET_PATH_LIMIT = 100

function identity(value: string): string {
	return createHash("sha256").update(value).digest("hex").slice(0, 16)
}

/** One stable daemon address per Electron user-data directory. */
export function socketPathFor(
	userDataDir: string,
	platform: NodeJS.Platform = process.platform,
	tempDir: string = tmpdir(),
): string {
	const suffix = identity(userDataDir)
	if (platform === "win32") return `\\\\.\\pipe\\simplex-${suffix}`

	const preferred = join(userDataDir, "simplex.sock")
	if (Buffer.byteLength(preferred) <= UNIX_SOCKET_PATH_LIMIT) return preferred

	const fallback = join(tempDir, `simplex-${suffix}.sock`)
	if (Buffer.byteLength(fallback) <= UNIX_SOCKET_PATH_LIMIT) return fallback

	const finalFallback = join("/tmp", `simplex-${suffix}.sock`)
	if (Buffer.byteLength(finalFallback) <= UNIX_SOCKET_PATH_LIMIT) return finalFallback
	throw new Error(`Could not derive a Simplex socket path within ${UNIX_SOCKET_PATH_LIMIT} bytes`)
}

export type ResourcePaths = { node: string; solver: string; ui: string }

export function resourcePaths(options: {
	isPackaged: boolean
	resourcesPath: string
	packageRoot: string
	simplexPackageRoot: string
	platform?: NodeJS.Platform
	arch?: string
}): ResourcePaths {
	const platform = options.platform ?? process.platform
	const arch = options.arch ?? process.arch
	const executable = platform === "win32" ? "node.exe" : "node"
	const path = platform === "win32" ? win32 : posix
	const node = options.isPackaged
		? path.join(options.resourcesPath, "runtime", executable)
		: path.join(options.packageRoot, "resources", "node", `${platform}-${arch}`, executable)
	const solver = options.isPackaged
		? path.join(options.resourcesPath, "simplex", "dist", "bin", "simplex.js")
		: path.join(options.simplexPackageRoot, "dist", "bin", "simplex.js")
	const ui = options.isPackaged
		? path.join(options.resourcesPath, "simplex", "dist", "ui", "index.html")
		: path.join(options.simplexPackageRoot, "dist", "ui", "index.html")
	return { node, solver, ui }
}

export function assertResources(paths: ResourcePaths): void {
	for (const [kind, path] of Object.entries(paths)) {
		if (!existsSync(path)) {
			throw new Error(`The staged Simplex ${kind} resource is missing: ${path}`)
		}
	}
}
