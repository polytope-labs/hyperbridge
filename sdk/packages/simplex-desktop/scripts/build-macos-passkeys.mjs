import { existsSync, mkdirSync, readFileSync, realpathSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { homedir } from "node:os"
import { fileURLToPath } from "node:url"
import { spawnSync } from "node:child_process"

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..")
export function buildMacPasskeys(arch = process.arch) {
	if (process.platform !== "darwin") throw new Error("Build the passkey module on macOS")
	if (arch === "universal") {
		const slices = [buildMacPasskeys("arm64"), buildMacPasskeys("x64")]
		const output = join(root, "build", "native", "darwin-universal", "mac-passkeys.node")
		mkdirSync(dirname(output), { recursive: true })
		const result = spawnSync("xcrun", ["lipo", "-create", ...slices, "-output", output], { stdio: "inherit" })
		if (result.error || result.status !== 0) throw new Error("Could not assemble universal Mac passkey module")
		return output
	}
	if (!["arm64", "x64"].includes(arch)) throw new Error("Unsupported Mac passkey architecture")
	const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).devDependencies.electron
	const includes = [
		process.env.NODE_API_INCLUDE,
		join(homedir(), ".electron-gyp", version, "include", "node"),
		join(dirname(dirname(realpathSync(process.execPath))), "include", "node"),
	].filter(Boolean)
	const include = includes.find((path) => existsSync(join(path, "node_api.h")))
	if (!include)
		throw new Error(
			"Node-API headers are missing. Set NODE_API_INCLUDE to a directory containing node_api.h (Node >=24 or the pinned Electron headers).",
		)
	const output = join(root, "build", "native", `darwin-${arch}`, "mac-passkeys.node")
	mkdirSync(dirname(output), { recursive: true })
	const result = spawnSync(
		"xcrun",
		[
			"clang++",
			"-std=c++17",
			"-fobjc-arc",
			"-fblocks",
			"-mmacosx-version-min=13.0",
			"-arch",
			arch === "x64" ? "x86_64" : arch,
			"-DNAPI_VERSION=8",
			"-DNODE_GYP_MODULE_NAME=mac_passkeys",
			"-I",
			include,
			"-bundle",
			"-undefined",
			"dynamic_lookup",
			"-framework",
			"Foundation",
			"-framework",
			"AppKit",
			"-framework",
			"AuthenticationServices",
			"-framework",
			"Security",
			join(root, "native", "macos", "passkeys.mm"),
			"-o",
			output,
		],
		{ stdio: "inherit" },
	)
	if (result.error) throw result.error
	if (result.status !== 0) throw new Error("Mac passkey native compilation failed")
	return output
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	if (process.platform === "darwin") process.stdout.write(`${buildMacPasskeys(process.argv[2])}\n`)
}
