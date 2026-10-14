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
	noExternal: [
		"electron-updater",
		"@hyperbridge/simplex/config-storage",
		"@hyperbridge/simplex/error-reporting",
		"@simplewebauthn/server",
		/^@sentry\//,
	],
	// Release builds bake Sentry DSNs in; local builds leave them empty and never report.
	// The UI DSN only widens the renderer CSP to that project's ingest origin.
	define: {
		__SIMPLEX_DESKTOP_SENTRY_DSN__: JSON.stringify(process.env.SIMPLEX_DESKTOP_SENTRY_DSN ?? ""),
		__SIMPLEX_UI_SENTRY_DSN__: JSON.stringify(process.env.SIMPLEX_UI_SENTRY_DSN ?? ""),
	},
	// electron-updater is CommonJS and performs runtime Electron imports. ESM
	// main processes do not expose require unless we create it explicitly.
	banner: {
		js: "import { createRequire as __desktopCreateRequire } from 'node:module'; const require = __desktopCreateRequire(import.meta.url);",
	},
})
