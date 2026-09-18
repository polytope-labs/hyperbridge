import { readFileSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"
import { parse } from "yaml"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const workspaceRoot = resolve(packageRoot, "../..")
const repositoryRoot = resolve(workspaceRoot, "..")

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
	it("keeps the released application identity and platform icons stable and tracks the solver version", () => {
		const builder = readYaml(join(packageRoot, "electron-builder.yml"))
		const desktop = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")) as {
			version: string
		}
		const simplex = JSON.parse(readFileSync(join(packageRoot, "../simplex/package.json"), "utf8")) as {
			version: string
		}

		expect(builder).toMatchObject({
			appId: "network.hyperbridge.simplex",
			productName: "Simplex",
			mac: { icon: "resources/icon.icns" },
			win: { icon: "resources/icon.ico" },
			linux: { icon: "resources/icons" },
		})
		expect(desktop.version).toBe(simplex.version)
	})

	it("keeps build policy in the supported workspace configuration", () => {
		const workspace = readYaml(join(workspaceRoot, "pnpm-workspace.yaml"))

		expect(workspace.allowBuilds).toMatchObject({ "utf-8-validate": false })
		expect(workspace.minimumReleaseAgeExclude).toContain("electron@44.3.0")
		expect(workspace.overrides).toEqual({
			"@hyperbridge/simplex>pino": "~10.3.1",
			"@hyperbridge/simplex>vite": "8.0.16",
			"axios@<1": ">=0.33.0 <1",
			"axios@>=1": "^1.18.0",
			viem: "2.47.6",
			vite: "6.4.2",
		})
	})

	it("budgets for the installed desktop app include required license files", () => {
		const workflow = readYaml(join(repositoryRoot, ".github/workflows/publish-simplex-desktop.yml")) as {
			jobs?: {
				build?: {
					strategy?: { matrix?: { include?: Array<{ target: string; size_budget_mib: number }> } }
				}
			}
		}
		const budgets = Object.fromEntries(
			(workflow.jobs?.build?.strategy?.matrix?.include ?? []).map(({ target, size_budget_mib }) => [
				target,
				size_budget_mib,
			]),
		)

		expect(budgets).toEqual({
			"darwin-arm64": 520,
			"darwin-x64": 520,
			"win32-x64": 580,
			"linux-x64": 520,
			"linux-arm64": 520,
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
