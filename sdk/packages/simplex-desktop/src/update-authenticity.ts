import { spawnSync } from "node:child_process"
import { readFileSync } from "node:fs"
import { join, posix } from "node:path"
import { parse } from "yaml"

export interface UpdateAuthenticity {
	enabled: boolean
	reason?: string
}

const APPLE_TEAM_ID = /^[A-Z0-9]{6,20}$/

function publisherNames(config: unknown): string[] {
	if (!config || typeof config !== "object") return []
	const value = (config as { publisherName?: unknown }).publisherName
	const names = Array.isArray(value) ? value : [value]
	return names.filter((name): name is string => typeof name === "string" && name.trim().length > 0)
}

/** Resolve a macOS bundle with macOS path semantics, even in cross-platform tests. */
export function macApplicationBundlePath(executablePath: string): string {
	return posix.resolve(posix.dirname(executablePath), "../..")
}

/**
 * Return the signing requirement used for a distributed macOS build. The Team
 * ID is stored in the signed app metadata by the release build; it is public,
 * but an attacker cannot change it without invalidating the application seal.
 */
export function macCodeSigningRequirement(teamId: string): string | undefined {
	if (!APPLE_TEAM_ID.test(teamId)) return undefined
	return `=anchor apple generic and certificate leaf[subject.OU] = ${teamId}`
}

export function macCodeSigningArguments(bundlePath: string, requirement: string): string[] {
	return ["--verify", "--strict", "-R", requirement, bundlePath]
}

export function macTeamIdFromAppPackage(appPath: string): string | undefined {
	try {
		const manifest = JSON.parse(readFileSync(join(appPath, "package.json"), "utf8")) as { simplexMacTeamId?: unknown }
		return typeof manifest.simplexMacTeamId === "string" && APPLE_TEAM_ID.test(manifest.simplexMacTeamId)
			? manifest.simplexMacTeamId
			: undefined
	} catch {
		return undefined
	}
}

export function updateAuthenticityForInstallation(options: {
	packaged: boolean
	platform: NodeJS.Platform
	resourcesPath: string
	executablePath: string
	expectedMacTeamId?: string
	verifyMacSignature?: (bundlePath: string, requirement: string) => boolean
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
		const bundlePath = macApplicationBundlePath(options.executablePath)
		const requirement = options.expectedMacTeamId ? macCodeSigningRequirement(options.expectedMacTeamId) : undefined
		if (!requirement) {
			return { enabled: false, reason: "The installed macOS application has no trusted Apple Team ID" }
		}
		const verify =
			options.verifyMacSignature ??
			((path: string, signingRequirement: string) =>
				spawnSync("/usr/bin/codesign", macCodeSigningArguments(path, signingRequirement), {
					stdio: "ignore",
				}).status === 0)
		return verify(bundlePath, requirement)
			? { enabled: true }
			: { enabled: false, reason: "The installed macOS application is not code signed" }
	}

	return {
		enabled: false,
		reason: "Linux automatic updates require independently signed release metadata",
	}
}
