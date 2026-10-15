import { readFileSync } from "fs"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"
import { sentryVitePlugin } from "@sentry/vite-plugin"
import { resolve } from "path"

const outDir = resolve(__dirname, "../dist/ui")
const { version } = JSON.parse(readFileSync(resolve(__dirname, "../package.json"), "utf8")) as { version: string }
const release = `simplex-ui@${version}`

// Release builds set these in CI. Without a DSN the UI never reports; without
// upload credentials the build ships no source maps at all.
const sentryDsn = process.env.SIMPLEX_UI_SENTRY_DSN ?? ""
const uploadSourceMaps = Boolean(
	process.env.SENTRY_AUTH_TOKEN && process.env.SENTRY_ORG && process.env.SIMPLEX_UI_SENTRY_PROJECT,
)

export default defineConfig({
	root: __dirname,
	base: "./",
	plugins: [
		react(),
		tailwindcss(),
		uploadSourceMaps &&
			sentryVitePlugin({
				authToken: process.env.SENTRY_AUTH_TOKEN,
				org: process.env.SENTRY_ORG,
				project: process.env.SIMPLEX_UI_SENTRY_PROJECT,
				release: { name: release },
				sourcemaps: { filesToDeleteAfterUpload: [resolve(outDir, "**/*.map")] },
				telemetry: false,
			}),
	],
	define: {
		__SIMPLEX_UI_SENTRY_DSN__: JSON.stringify(sentryDsn),
		__SIMPLEX_UI_RELEASE__: JSON.stringify(release),
	},
	resolve: {
		// ui/src may import browser-safe modules from src/; the build fails if one grows a node-only import
		alias: { "@": resolve(__dirname, "../src") },
	},
	build: {
		outDir,
		emptyOutDir: true,
		// Maps are uploaded to Sentry and deleted, never shipped with the UI.
		sourcemap: uploadSourceMaps ? "hidden" : false,
	},
	server: {
		port: 5173,
		proxy: {
			"/api": "http://127.0.0.1:8686",
			"/health": "http://127.0.0.1:8686",
		},
	},
})
