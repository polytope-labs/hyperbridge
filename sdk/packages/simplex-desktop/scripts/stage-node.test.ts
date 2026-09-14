import { mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import * as openpgp from "openpgp"
import * as tar from "tar"
import { describe, expect, it } from "vitest"
import {
	assetFor,
	atomicInstall,
	checksumFromManifest,
	defaultTarget,
	extractNode,
	mergeUniversal,
	NODE_RELEASE_KEY_FINGERPRINT,
	NODE_VERSION,
	verifyManifestSignature,
} from "./stage-node.mjs"

describe("Node runtime staging", () => {
	it("maps every supported archive without falling back to PATH", () => {
		expect(assetFor("darwin-arm64")).toBe(`node-v${NODE_VERSION}-darwin-arm64.tar.gz`)
		expect(assetFor("darwin-x64")).toBe(`node-v${NODE_VERSION}-darwin-x64.tar.gz`)
		expect(assetFor("linux-arm64")).toBe(`node-v${NODE_VERSION}-linux-arm64.tar.gz`)
		expect(assetFor("linux-x64")).toBe(`node-v${NODE_VERSION}-linux-x64.tar.gz`)
		expect(assetFor("win32-x64")).toBe("win-x64/node.exe")
		expect(() => defaultTarget("freebsd", "x64")).toThrow(/No bundled Node runtime target/)
	})

	it("requires exactly one exact checksum entry", () => {
		const hash = "a".repeat(64)
		expect(checksumFromManifest(`${hash}  node.tar.gz\n`, "node.tar.gz")).toBe(hash)
		expect(() => checksumFromManifest(`${hash}  other-node.tar.gz\n`, "node.tar.gz")).toThrow(/0 entries/)
		expect(() => checksumFromManifest(`${hash}  node.tar.gz\n${hash}  node.tar.gz\n`, "node.tar.gz")).toThrow(
			/2 entries/,
		)
	})

	it("accepts a valid clearsigned manifest and rejects tampering", async () => {
		const { privateKey, publicKey } = await openpgp.generateKey({
			type: "ecc",
			curve: "ed25519",
			userIDs: [{ name: "Simplex staging test" }],
		})
		const signingKey = await openpgp.readPrivateKey({ armoredKey: privateKey })
		const testKey = await openpgp.readKey({ armoredKey: publicKey })
		const message = await openpgp.createCleartextMessage({ text: `${"b".repeat(64)}  node.tar.gz\n` })
		const signed = await openpgp.sign({ message, signingKeys: signingKey })
		await expect(
			verifyManifestSignature(signed, publicKey, testKey.getFingerprint().toUpperCase()),
		).resolves.toContain("node.tar.gz")
		await expect(verifyManifestSignature(signed, publicKey, NODE_RELEASE_KEY_FINGERPRINT)).rejects.toThrow(
			/fingerprint does not match/,
		)
		await expect(verifyManifestSignature(signed.replace("node.tar.gz", "evil.tar.gz"), publicKey)).rejects.toThrow()
	})

	it("extracts only bin/node and rejects an archive without it", async () => {
		const directory = await mkdtemp(join(tmpdir(), "simplex-stage-archive-"))
		const root = `node-v${NODE_VERSION}-linux-x64`
		const source = join(directory, root)
		await mkdir(join(source, "bin"), { recursive: true })
		await mkdir(join(source, "include"), { recursive: true })
		await writeFile(join(source, "bin", "node"), "node-runtime")
		await writeFile(join(source, "bin", "npm"), "must-not-extract")
		await writeFile(join(source, "include", "node.h"), "must-not-extract")
		const archive = join(directory, `${root}.tar.gz`)
		await tar.c({ cwd: directory, file: archive, gzip: true }, [root])

		const output = join(directory, "staged-node")
		await extractNode(archive, output)
		expect(await readFile(output, "utf8")).toBe("node-runtime")
		await expect(readFile(join(directory, "npm"))).rejects.toMatchObject({ code: "ENOENT" })
		await expect(readFile(join(directory, "node.h"))).rejects.toMatchObject({ code: "ENOENT" })

		const wrongRoot = join(directory, "wrong-node")
		await mkdir(wrongRoot)
		await writeFile(join(wrongRoot, "README"), "not a runtime")
		const wrongArchive = join(directory, "wrong-node.tar.gz")
		await tar.c({ cwd: directory, file: wrongArchive, gzip: true }, ["wrong-node"])
		await expect(extractNode(wrongArchive, join(directory, "missing-node"))).rejects.toThrow(
			/did not contain bin\/node/,
		)
	})

	it("extracts independently when verified slices are staged in parallel", async () => {
		const directory = await mkdtemp(join(tmpdir(), "simplex-stage-parallel-"))
		const archives = await Promise.all(
			Array.from({ length: 12 }, async (_, index) => {
				const root = `node-slice-${index}`
				await mkdir(join(directory, root, "bin"), { recursive: true })
				await writeFile(join(directory, root, "bin", "node"), `runtime-${index}-${"x".repeat(64_000)}`)
				const archive = join(directory, `${root}.tar.gz`)
				await tar.c({ cwd: directory, file: archive, gzip: true }, [root])
				return archive
			}),
		)
		const destinations = archives.map((_, index) => join(directory, `verified-node-${index}`))

		await Promise.all(archives.map((archive, index) => extractNode(archive, destinations[index])))

		for (let index = 0; index < destinations.length; index += 1) {
			expect(await readFile(destinations[index], "utf8")).toBe(`runtime-${index}-${"x".repeat(64_000)}`)
		}
	})

	it("installs runtimes atomically without moving the verified source across filesystems", async () => {
		const directory = await mkdtemp(join(tmpdir(), "simplex-stage-install-"))
		const source = join(directory, "verified-node")
		const destination = join(directory, "runtime", "node")
		await mkdir(join(directory, "runtime"))
		await writeFile(destination, "old-runtime")
		await writeFile(source, "verified-runtime", { mode: 0o600 })
		await atomicInstall(source, destination, true)
		expect(await readFile(destination, "utf8")).toBe("verified-runtime")
		expect(await readFile(source, "utf8")).toBe("verified-runtime")
		// Windows executability is determined by the `.exe` file type rather than
		// POSIX mode bits. Unix staging must still install an executable runtime.
		if (process.platform !== "win32") {
			expect((await stat(destination)).mode & 0o777).toBe(0o755)
		}
	})

	it("builds a universal runtime from the two independently staged slices", () => {
		let command: unknown[] | undefined
		mergeUniversal("/verified/arm64", "/verified/x64", "/output/node", (...args: unknown[]) => {
			command = args
			return { status: 0, stdout: "", stderr: "" }
		})
		expect(command).toEqual([
			"/usr/bin/lipo",
			["-create", "/verified/arm64", "/verified/x64", "-output", "/output/node"],
			{ encoding: "utf8" },
		])
	})
})
