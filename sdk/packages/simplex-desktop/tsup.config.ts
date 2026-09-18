import { defineConfig } from "tsup"

export default defineConfig({
	entry: { main: "src/main.ts" },
	format: ["esm"],
	platform: "node",
	target: "node24",
	bundle: true,
	clean: true,
	sourcemap: true,
	// Electron is supplied by the host runtime. Bundle electron-updater so the
	// packaged main process has no undeclared dependency on a workspace symlink.
	external: ["electron"],
	noExternal: ["electron-updater"],
	// electron-updater is CommonJS and performs runtime Electron imports. ESM
	// main processes do not expose require unless we create it explicitly.
	banner: { js: "import { createRequire } from 'node:module'; const require = createRequire(import.meta.url);" },
})
