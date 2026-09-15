import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { describe, expect, it } from "vitest"
import {
	assertResources,
	resourcePaths,
	socketPathFor,
	UNIX_SOCKET_PATH_LIMIT,
	userDataOverrideFromArgv,
} from "../desktop-paths"

describe("desktop paths", () => {
	it("applies an explicit Electron user-data directory to desktop state", () => {
		expect(userDataOverrideFromArgv(["electron", ".", "--user-data-dir=./profile"])).toBe(resolve("./profile"))
		expect(userDataOverrideFromArgv(["electron", "."])).toBeUndefined()
		expect(() => userDataOverrideFromArgv(["electron", ".", "--user-data-dir="])).toThrow(/requires a directory/)
	})
	it("uses POSIX paths for a short Unix socket regardless of the build host", () => {
		expect(socketPathFor("/tmp/simplex-user", "darwin", "/tmp")).toBe("/tmp/simplex-user/simplex.sock")
	})

	it("falls back to a stable short POSIX socket path", () => {
		const userData = `/Users/operator/${"long-directory/".repeat(10)}`
		const first = socketPathFor(userData, "darwin", "/tmp")
		const second = socketPathFor(userData, "darwin", "/tmp")
		expect(first).toBe(second)
		expect(first).toMatch(/^\/tmp\/simplex-[0-9a-f]{16}\.sock$/)
		expect(Buffer.byteLength(first)).toBeLessThanOrEqual(UNIX_SOCKET_PATH_LIMIT)
	})

	it("uses a deterministic Windows named pipe", () => {
		expect(socketPathFor("C:\\Users\\operator\\Simplex", "win32")).toMatch(/^\\\\\.\\pipe\\simplex-[0-9a-f]{16}$/)
	})

	it("resolves development and packaged resources without PATH", () => {
		expect(
			resourcePaths({
				isPackaged: false,
				resourcesPath: "/unused",
				packageRoot: "/workspace/simplex-desktop",
				simplexPackageRoot: "/workspace/simplex",
				platform: "linux",
				arch: "arm64",
			}),
		).toEqual({
			node: "/workspace/simplex-desktop/resources/node/linux-arm64/node",
			solver: "/workspace/simplex/dist/bin/simplex.js",
			ui: "/workspace/simplex/dist/ui/index.html",
		})

		expect(
			resourcePaths({
				isPackaged: true,
				resourcesPath: "C:\\app\\resources",
				packageRoot: "unused",
				simplexPackageRoot: "unused",
				platform: "win32",
				arch: "x64",
			}),
		).toEqual({
			node: "C:\\app\\resources\\runtime\\node.exe",
			solver: "C:\\app\\resources\\simplex\\dist\\bin\\simplex.js",
			ui: "C:\\app\\resources\\simplex\\dist\\ui\\index.html",
		})
	})

	it("fails before launch when a resource is missing", () => {
		const directory = mkdtempSync(join(tmpdir(), "simplex-desktop-resources-"))
		const node = join(directory, "node")
		const solver = join(directory, "simplex.js")
		const ui = join(directory, "index.html")
		writeFileSync(node, "node")
		expect(() => assertResources({ node, solver, ui })).toThrow(/solver resource is missing/)
		mkdirSync(join(directory, "nested"), { recursive: true })
		writeFileSync(solver, "solver")
		expect(() => assertResources({ node, solver, ui })).toThrow(/ui resource is missing/)
		writeFileSync(ui, "ui")
		expect(() => assertResources({ node, solver, ui })).not.toThrow()
	})
})
