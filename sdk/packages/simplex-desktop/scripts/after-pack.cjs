module.exports = async function afterPack(context) {
	const { createRequire } = require("node:module")
	const { resolve } = require("node:path")
	const toolingRequire = createRequire(resolve(__dirname, "../tooling/package.json"))
	const { Arch } = toolingRequire("builder-util")
	const { installPackagedRuntime } = await import("./package-layout.mjs")
	await installPackagedRuntime({
		appOutDir: context.appOutDir,
		platform: context.electronPlatformName,
		arch: Arch[context.arch],
		productFilename: context.packager.appInfo.productFilename,
	})
}
