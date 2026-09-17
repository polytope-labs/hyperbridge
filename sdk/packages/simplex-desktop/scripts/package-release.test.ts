import { createHash } from "node:crypto"
import { mkdir, mkdtemp, readFile, unlink, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it } from "vitest"
import {
	assembleRelease,
	mergeUpdateMetadata,
	parseUpdateMetadata,
	stringifyUpdateMetadata,
} from "./assemble-release.mjs"
import { assertReleaseAssets } from "./assert-release-assets.mjs"
import {
	assertPackagedResources,
	EXTERNAL_RUNTIME_PACKAGES,
	packagedResourcesDirectory,
	runtimeTarget,
} from "./package-layout.mjs"
import { assertInstalledSize, installedAppDirectories } from "./package-size.mjs"
import { assertPackagingNodeVersion, normalizeBuilderArguments } from "./run-builder.mjs"
import { verifyReleaseTag } from "./verify-release-tag.mjs"
import { artifactNamesForPlatform, isUnavailableAppImageFuse } from "./e2e/artifact-smoke.mjs"

function sha512(value: string): string {
	return createHash("sha512").update(value).digest("base64")
}

describe("desktop package and release layout", () => {
	it("selects every launchable installer for the host platform", () => {
		const names = ["Simplex.dmg", "Simplex.zip", "Simplex.exe", "Simplex.AppImage", "Simplex.deb"]
		expect(artifactNamesForPlatform(names, "darwin")).toEqual(["Simplex.dmg", "Simplex.zip"])
		expect(artifactNamesForPlatform(names, "win32")).toEqual(["Simplex.exe"])
		expect(artifactNamesForPlatform(names, "linux")).toEqual(["Simplex.AppImage", "Simplex.deb"])
		expect(() => artifactNamesForPlatform(["Simplex.dmg"], "darwin")).toThrow(/updater ZIP/)
	})

	it("falls back only for the hosted runner's known AppImage FUSE failure", () => {
		expect(isUnavailableAppImageFuse(new Error("fusermount3: mount failed: Operation not permitted"))).toBe(true)
		expect(isUnavailableAppImageFuse(new Error("Simplex exited 1: application startup failed"))).toBe(false)
	})

	it("forwards pnpm-delimited builder target arguments", () => {
		expect(
			normalizeBuilderArguments([
				"--config",
				"electron-builder.yml",
				"--",
				"--linux",
				"--arm64",
				"--publish",
				"never",
			]),
		).toEqual(["--config", "electron-builder.yml", "--linux", "--arm64", "--publish", "never"])
		expect(normalizeBuilderArguments(["--win", "--x64"])).toEqual(["--win", "--x64"])
	})

	it("requires Node 24 for the release toolchain", () => {
		expect(() => assertPackagingNodeVersion("23.11.0")).toThrow(/requires Node 24/)
		expect(() => assertPackagingNodeVersion("24.19.0")).not.toThrow()
	})

	it("maps only supported package runtimes", () => {
		expect(runtimeTarget("darwin", "arm64")).toBe("darwin-arm64")
		expect(runtimeTarget("darwin", "x64")).toBe("darwin-x64")
		expect(runtimeTarget("linux", "arm64")).toBe("linux-arm64")
		expect(runtimeTarget("linux", "x64")).toBe("linux-x64")
		expect(runtimeTarget("win32", "x64")).toBe("win32-x64")
		expect(() => runtimeTarget("win32", "arm64")).toThrow(/No packaged Simplex runtime/)
	})

	it("verifies the complete packaged resource contract", async () => {
		const output = await mkdtemp(join(tmpdir(), "simplex-package-layout-"))
		const resources = packagedResourcesDirectory({
			appOutDir: output,
			platform: "linux",
			productFilename: "Simplex",
		})
		const required = [
			"runtime/node",
			"simplex/package.json",
			"simplex/dist/bin/simplex.js",
			"simplex/dist/ui/index.html",
			...EXTERNAL_RUNTIME_PACKAGES.map((name) => `node_modules/${name}/package.json`),
		]
		for (const path of required) {
			await mkdir(join(resources, path, ".."), { recursive: true })
			await writeFile(join(resources, path), path.endsWith(".json") ? "{}" : "fixture")
		}
		await expect(assertPackagedResources(resources, "linux")).resolves.toBeUndefined()
		await expect(assertPackagedResources(resources, "win32")).rejects.toThrow()
	})

	it("enforces the installed-size budget against unpacked apps", async () => {
		const output = await mkdtemp(join(tmpdir(), "simplex-package-size-"))
		const app = join(output, "linux-unpacked")
		await mkdir(app)
		await writeFile(join(app, "simplex"), Buffer.alloc(1024))
		expect(await installedAppDirectories(output)).toEqual([app])
		await expect(assertInstalledSize(output, 1)).resolves.toMatchObject([{ app }])
		await expect(assertInstalledSize(output, 0.0001)).rejects.toThrow(/budget/)
	})

	it("merges architecture-specific updater metadata without losing checksums", () => {
		const merged = mergeUpdateMetadata([
			{ version: "1.2.3", files: [{ url: "Simplex-1.2.3-mac-arm64.zip", sha512: "arm" }] },
			{ version: "1.2.3", files: [{ url: "Simplex-1.2.3-mac-x64.zip", sha512: "intel" }] },
		])
		expect(merged.files).toEqual([
			{ url: "Simplex-1.2.3-mac-arm64.zip", sha512: "arm" },
			{ url: "Simplex-1.2.3-mac-x64.zip", sha512: "intel" },
		])
		expect(() =>
			mergeUpdateMetadata([
				{ version: "1.2.3", files: [{ url: "same.zip", sha512: "one" }] },
				{ version: "1.2.3", files: [{ url: "same.zip", sha512: "two" }] },
			]),
		).toThrow(/Duplicate/)
	})

	it("assembles release assets and one merged macOS channel file", async () => {
		const root = await mkdtemp(join(tmpdir(), "simplex-release-input-"))
		const output = join(root, "output")
		for (const [arch, hash] of [
			["arm64", "arm"],
			["x64", "intel"],
		]) {
			const directory = join(root, `darwin-${arch}`, "release")
			await mkdir(directory, { recursive: true })
			await writeFile(join(directory, `Simplex-1.2.3-mac-${arch}.zip`), arch)
			await writeFile(
				join(directory, "latest-mac.yml"),
				stringifyUpdateMetadata({
					version: "1.2.3",
					files: [{ url: `Simplex-1.2.3-mac-${arch}.zip`, sha512: hash }],
				}),
			)
		}
		const files = await assembleRelease(root, output)
		expect(files).toEqual(["Simplex-1.2.3-mac-arm64.zip", "Simplex-1.2.3-mac-x64.zip", "latest-mac.yml"])
		const metadata = parseUpdateMetadata(await readFile(join(output, "latest-mac.yml"), "utf8"))
		expect(metadata.files).toHaveLength(2)
	})

	it.each([
		{ version: "1.2.3", channel: "latest" },
		{ version: "1.2.3-beta.4", channel: "beta" },
	])("requires every $channel platform installer and checksum-backed updater entry", async ({ version, channel }) => {
		const directory = await mkdtemp(join(tmpdir(), "simplex-release-assets-"))
		const artifacts = [
			`Simplex-${version}-mac-arm64.dmg`,
			`Simplex-${version}-mac-arm64.zip`,
			`Simplex-${version}-mac-x64.dmg`,
			`Simplex-${version}-mac-x64.zip`,
			`Simplex-${version}-win-x64.exe`,
			`Simplex-${version}-linux-x64.AppImage`,
			`Simplex-${version}-linux-x64.deb`,
			`Simplex-${version}-linux-arm64.AppImage`,
			`Simplex-${version}-linux-arm64.deb`,
			`Simplex-${version}-mac-arm64.zip.blockmap`,
			`Simplex-${version}-mac-x64.zip.blockmap`,
			`Simplex-${version}-win-x64.exe.blockmap`,
		]
		for (const name of artifacts) await writeFile(join(directory, name), name)
		const metadata = {
			[`${channel}.yml`]: [`Simplex-${version}-win-x64.exe`],
			[`${channel}-mac.yml`]: [`Simplex-${version}-mac-arm64.zip`, `Simplex-${version}-mac-x64.zip`],
			[`${channel}-linux.yml`]: [`Simplex-${version}-linux-x64.AppImage`, `Simplex-${version}-linux-x64.deb`],
			[`${channel}-linux-arm64.yml`]: [
				`Simplex-${version}-linux-arm64.AppImage`,
				`Simplex-${version}-linux-arm64.deb`,
			],
		}
		for (const [name, urls] of Object.entries(metadata)) {
			await writeFile(
				join(directory, name),
				stringifyUpdateMetadata({ version, files: urls.map((url) => ({ url, sha512: sha512(url) })) }),
			)
		}

		await expect(assertReleaseAssets(directory, version)).resolves.toBeUndefined()
		await writeFile(join(directory, `Simplex-${version}-mac-arm64.zip`), "tampered")
		await expect(assertReleaseAssets(directory, version)).rejects.toThrow(/invalid SHA-512/)
		await writeFile(join(directory, `Simplex-${version}-mac-arm64.zip`), `Simplex-${version}-mac-arm64.zip`)
		await writeFile(
			join(directory, `${channel}-linux.yml`),
			stringifyUpdateMetadata({
				version,
				files: [
					{
						url: `Simplex-${version}-win-x64.exe`,
						sha512: sha512(`Simplex-${version}-win-x64.exe`),
					},
				],
			}),
		)
		await expect(assertReleaseAssets(directory, version)).rejects.toThrow(/does not reference/)
		await writeFile(
			join(directory, `${channel}-linux.yml`),
			stringifyUpdateMetadata({
				version,
				files: metadata[`${channel}-linux.yml`].map((url) => ({ url, sha512: sha512(url) })),
			}),
		)
		await unlink(join(directory, `Simplex-${version}-linux-arm64.deb`))
		await expect(assertReleaseAssets(directory, version)).rejects.toThrow()
	})

	it("requires the release tag to match the package version and namespace", async () => {
		await expect(verifyReleaseTag("simplex-desktop-v0.16.2")).resolves.toBe("0.16.2")
		await expect(verifyReleaseTag("simplex-v0.16.2")).rejects.toThrow(/must exactly match/)
	})
})
