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

		expect(workspace.allowBuilds).toMatchObject({ "utf-8-validate": false })
		expect(workspace.minimumReleaseAgeExclude).toContain("electron@44.3.0")
		expect(workspace.overrides).toEqual({
			"@hyperbridge/simplex>pino": "~10.3.1",
			"@hyperbridge/simplex>vite": "8.0.16",
			axios: "1.13.6",
			viem: "2.47.6",
			vite: "6.4.2",
		})
	})

	it("keeps every frozen-lockfile importer aligned with its package manifest", () => {
		const workspace = readYaml(join(workspaceRoot, "pnpm-workspace.yaml")) as {
			overrides?: Record<string, string>
		}
		const lockfile = readYaml(join(workspaceRoot, "pnpm-lock.yaml")) as {
			importers?: Record<string, Record<string, Record<string, { specifier?: string }>>>
		}
		for (const [importer, locked] of Object.entries(lockfile.importers ?? {})) {
			const manifestPath = join(workspaceRoot, importer, "package.json")
			const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Record<string, unknown>
			const expected = dependencySpecifiers(manifest)
			const packageName = manifest.name
			for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
				for (const [name, entry] of Object.entries(locked[section] ?? {})) {
					expect(entry.specifier, `${importer}:${name}`).toBe(
						workspace.overrides?.[`${packageName}>${name}`] ??
							workspace.overrides?.[name] ??
							expected[name],
					)
				}
			}
		}
	})
})
