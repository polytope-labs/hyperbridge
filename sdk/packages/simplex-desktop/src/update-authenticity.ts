import { spawnSync } from "node:child_process"
import { readFileSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { parse } from "yaml"

export interface UpdateAuthenticity {
	enabled: boolean
	reason?: string
}

function publisherNames(config: unknown): string[] {
	if (!config || typeof config !== "object") return []
	const value = (config as { publisherName?: unknown }).publisherName
	const names = Array.isArray(value) ? value : [value]
	return names.filter((name): name is string => typeof name === "string" && name.trim().length > 0)
}

export function updateAuthenticityForInstallation(options: {
	packaged: boolean
	platform: NodeJS.Platform
	resourcesPath: string
	executablePath: string
	verifyMacSignature?: (bundlePath: string) => boolean
}): UpdateAuthenticity {
	if (!options.packaged) return { enabled: false, reason: "Updates are disabled in development builds" }

	if (options.platform === "win32") {
		try {
			const config = parse(readFileSync(join(options.resourcesPath, "app-update.yml"), "utf8"))
			if (publisherNames(config).length > 0) return { enabled: true }
			return { enabled: false, reason: "The installed build has no trusted Windows publisher" }
		} catch (error) {
			return {
				enabled: false,
				reason: `The Windows update trust configuration is unavailable: ${error instanceof Error ? error.message : String(error)}`,
			}
		}
	}

	if (options.platform === "darwin") {
		const bundlePath = resolve(dirname(options.executablePath), "../..")
		const verify =
			options.verifyMacSignature ??
			((path: string) =>
				spawnSync("/usr/bin/codesign", ["--verify", "--deep", "--strict", path], { stdio: "ignore" }).status ===
				0)
		return verify(bundlePath)
			? { enabled: true }
			: { enabled: false, reason: "The installed macOS application is not code signed" }
	}

	return {
		enabled: false,
		reason: "Linux automatic updates require independently signed release metadata",
	}
}
