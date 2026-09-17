import { readFileSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"
import { parse } from "yaml"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const workspaceRoot = resolve(packageRoot, "../..")

function readYaml(path: string): Record<string, unknown> {
	return parse(readFileSync(path, "utf8")) as Record<string, unknown>
}

function dependencySpecifiers(manifest: Record<string, unknown>): Record<string, string> {
	const sections = ["dependencies", "devDependencies", "optionalDependencies", "peerDependencies"]
	return Object.assign(
		{},
		...sections.map((section) => {
			const value = manifest[section]
			return value && typeof value === "object" ? value : {}
		}),
	)
}

describe("desktop workspace package policy", () => {
	it("keeps build policy in the supported workspace configuration", () => {
		const workspace = readYaml(join(workspaceRoot, "pnpm-workspace.yaml"))
		const manifest = JSON.parse(readFileSync(join(workspaceRoot, "package.json"), "utf8")) as Record<string, unknown>

		expect(workspace.allowBuilds).toMatchObject({ "utf-8-validate": false })
		expect(workspace.minimumReleaseAgeExclude).toEqual(
			expect.arrayContaining([
				"@base-ui/react@1.8.0",
				"@base-ui/utils@0.4.0",
				"electron@44.3.0",
				"playwright-core@1.63.0",
			]),
		)
		expect(workspace).not.toHaveProperty("overrides")
		expect(manifest).not.toHaveProperty("pnpm")
	})

	it("keeps every frozen-lockfile importer aligned with its package manifest", () => {
		const lockfile = readYaml(join(workspaceRoot, "pnpm-lock.yaml")) as {
			importers?: Record<string, Record<string, Record<string, { specifier?: string }>>>
		}
		for (const [importer, locked] of Object.entries(lockfile.importers ?? {})) {
			const manifestPath = join(workspaceRoot, importer, "package.json")
			const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Record<string, unknown>
			const expected = dependencySpecifiers(manifest)
			for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
				for (const [name, entry] of Object.entries(locked[section] ?? {})) {
					expect(entry.specifier, `${importer}:${name}`).toBe(expected[name])
				}
			}
		}
	})
})
