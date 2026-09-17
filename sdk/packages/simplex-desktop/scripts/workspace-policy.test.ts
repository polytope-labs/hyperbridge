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

describe("desktop workspace package policy", () => {
	it("keeps frozen-lockfile overrides aligned with the supported workspace configuration", () => {
		const workspace = readYaml(join(workspaceRoot, "pnpm-workspace.yaml"))
		const lockfile = readYaml(join(workspaceRoot, "pnpm-lock.yaml"))
		const manifest = JSON.parse(readFileSync(join(workspaceRoot, "package.json"), "utf8")) as Record<string, unknown>

		expect(workspace.allowBuilds).toMatchObject({ "utf-8-validate": false })
		expect(lockfile.overrides).toEqual(workspace.overrides)
		expect(manifest).not.toHaveProperty("pnpm")
	})
})
