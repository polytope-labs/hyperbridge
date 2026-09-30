import { readFileSync } from "node:fs"
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { afterEach, describe, expect, it, vi } from "vitest"
import { electronRuntimeFiles, ensureElectronRuntime } from "./ensure-electron.mjs"

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const temporaryDirectories: string[] = []

async function fixture(): Promise<string> {
	const directory = await mkdtemp(join(tmpdir(), "simplex-electron-runtime-"))
	temporaryDirectories.push(directory)
	return directory
}

afterEach(async () => {
	await Promise.all(
		temporaryDirectories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })),
	)
})

describe("Electron runtime prerequisite", () => {
	it("runs before packaging and Electron E2E", () => {
		const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")) as {
			scripts: Record<string, string>
		}
		expect(manifest.scripts.package).toMatch(/^pnpm run ensure:electron && /)
		expect(manifest.scripts["test:e2e"]).toMatch(/^pnpm run ensure:electron && /)
	})

	it("installs a missing runtime and verifies its executable and licenses", async () => {
		const electronRoot = await fixture()
		const runInstaller = vi.fn(async () => {
			for (const path of electronRuntimeFiles("linux")) {
				const artifact = join(electronRoot, "dist", path)
				await mkdir(dirname(artifact), { recursive: true })
				await writeFile(artifact, path)
			}
		})

		await expect(ensureElectronRuntime({ electronRoot, platform: "linux", runInstaller })).resolves.toBe(true)
		expect(runInstaller).toHaveBeenCalledOnce()
	})

	it("retries a transient Electron download failure", async () => {
		const electronRoot = await fixture()
		const runInstaller = vi
			.fn()
			.mockRejectedValueOnce(new Error("Response code 504 (Gateway Time-out)"))
			.mockImplementationOnce(async () => {
				for (const path of electronRuntimeFiles("darwin")) {
					const artifact = join(electronRoot, "dist", path)
					await mkdir(dirname(artifact), { recursive: true })
					await writeFile(artifact, path)
				}
			})
		const sleep = vi.fn(async () => undefined)

		await expect(ensureElectronRuntime({ electronRoot, platform: "darwin", runInstaller, sleep })).resolves.toBe(
			true,
		)
		expect(runInstaller).toHaveBeenCalledTimes(2)
		expect(sleep).toHaveBeenCalledOnce()
	})

	it("does not reinstall a complete runtime", async () => {
		const electronRoot = await fixture()
		for (const path of electronRuntimeFiles("win32")) {
			const artifact = join(electronRoot, "dist", path)
			await mkdir(dirname(artifact), { recursive: true })
			await writeFile(artifact, path)
		}
		const runInstaller = vi.fn()

		await expect(ensureElectronRuntime({ electronRoot, platform: "win32", runInstaller })).resolves.toBe(false)
		expect(runInstaller).not.toHaveBeenCalled()
	})

	it("fails when installation leaves the license bundle incomplete", async () => {
		const electronRoot = await fixture()

		await expect(
			ensureElectronRuntime({ electronRoot, platform: "darwin", runInstaller: async () => undefined }),
		).rejects.toThrow(/Electron runtime installation did not produce.*LICENSE/s)
	})
})
