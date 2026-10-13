module.exports = async function afterPack(context) {
	const { createRequire } = require("node:module")
	const { resolve } = require("node:path")
	const { signPackagedWindowsRuntime } = require("./release-signing.cjs")
	const toolingRequire = createRequire(resolve(__dirname, "../tooling/package.json"))
	const { Arch } = toolingRequire("builder-util")
	const { installPackagedRuntime } = await import("./package-layout.mjs")
	const runtimePath = await installPackagedRuntime({
		appOutDir: context.appOutDir,
		platform: context.electronPlatformName,
		arch: Arch[context.arch],
		productFilename: context.packager.appInfo.productFilename,
	})
	await signPackagedWindowsRuntime(runtimePath, context)
	if (context.electronPlatformName === "darwin") {
		const { buildMacPasskeys } = await import("./build-macos-passkeys.mjs")
		const { copyFile, mkdir } = require("node:fs/promises")
		const { dirname, join } = require("node:path")
		const destination = join(dirname(dirname(runtimePath)), "desktop", "native", "mac-passkeys.node")
		await mkdir(dirname(destination), { recursive: true })
		await copyFile(buildMacPasskeys(Arch[context.arch]), destination)
	}
}
