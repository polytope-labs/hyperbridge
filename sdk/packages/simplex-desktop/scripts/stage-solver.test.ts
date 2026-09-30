import { access, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { afterEach, describe, expect, it } from "vitest"
import { stageSolverDistribution } from "./stage-solver.mjs"

const temporaryDirectories: string[] = []

afterEach(async () => {
	await Promise.all(
		temporaryDirectories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })),
	)
})

describe("desktop solver staging", () => {
	it("copies runtime resources without library-only build artifacts", async () => {
		const root = await mkdtemp(join(tmpdir(), "simplex-stage-solver-"))
		temporaryDirectories.push(root)
		const source = join(root, "source")
		const output = join(root, "output")
		await Promise.all([
			mkdir(join(source, "dist/bin"), { recursive: true }),
			mkdir(join(source, "dist/ui"), { recursive: true }),
		])
		await Promise.all([
			writeFile(join(source, "package.json"), "{}"),
			writeFile(join(source, "dist/bin/simplex.js"), "solver"),
			writeFile(join(source, "dist/bin/simplex.js.map"), "{}"),
			writeFile(join(source, "dist/ui/index.html"), "ui"),
			writeFile(join(source, "dist/index.js"), "library"),
			writeFile(join(source, "dist/index.js.map"), "library map"),
		])

		await stageSolverDistribution(source, output)

		await expect(access(join(output, "package.json"))).resolves.toBeUndefined()
		await expect(access(join(output, "dist/bin/simplex.js"))).resolves.toBeUndefined()
		await expect(access(join(output, "dist/bin/simplex.js.map"))).resolves.toBeUndefined()
		await expect(access(join(output, "dist/ui/index.html"))).resolves.toBeUndefined()
		await expect(access(join(output, "dist/index.js"))).rejects.toMatchObject({ code: "ENOENT" })
		await expect(access(join(output, "dist/index.js.map"))).rejects.toMatchObject({ code: "ENOENT" })
	})

	it("rejects source maps that embed dependency source text", async () => {
		const root = await mkdtemp(join(tmpdir(), "simplex-stage-solver-map-"))
		temporaryDirectories.push(root)
		const source = join(root, "source")
		await Promise.all([
			mkdir(join(source, "dist/bin"), { recursive: true }),
			mkdir(join(source, "dist/ui"), { recursive: true }),
		])
		await Promise.all([
			writeFile(join(source, "package.json"), "{}"),
			writeFile(join(source, "dist/bin/simplex.js"), "solver"),
			writeFile(
				join(source, "dist/bin/simplex.js.map"),
				JSON.stringify({ version: 3, sources: ["source.ts"], sourcesContent: ["throw new Error()"] }),
			),
			writeFile(join(source, "dist/ui/index.html"), "ui"),
		])

		await expect(stageSolverDistribution(source, join(root, "output"))).rejects.toThrow(/embeds source content/)
	})
})
