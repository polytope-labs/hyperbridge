import { copyFileSync, existsSync, mkdirSync } from "node:fs"
import { dirname } from "node:path"
import { defineConfig } from "tsup"

export default defineConfig({
	entry: ["src/index.ts", "src/intents-helpers.ts"],
	outDir: "dist/node",
	format: ["esm", "cjs"],
	dts: true,
	sourcemap: true,
	platform: "node",
	clean: true,
	splitting: false,
	// `tronweb` is declared free of side effects so that an entry which uses none of it drops the
	// import outright. Rollup otherwise keeps a bare `import "tronweb"` in every entry of this build,
	// including `intents-helpers` — the entry that exists to stay out of TronWeb's way. Loading it
	// pulls axios and its https-proxy-agent, which loads `debug`, which deletes `process.env.DEBUG` as
	// it initialises; the SubQuery indexer runs mappings in a VM2 sandbox with a frozen `process`, so
	// that delete throws and kills the worker.
	treeshake: { moduleSideEffects: (id: string) => id !== "tronweb" },
	esbuildOptions: (esbuildOpt) => {
		esbuildOpt.alias = {
			"@/ckb-utils/web": "./src/utils/ckb-mmr-wasm/dist/node/node",
			"@/ckb-utils/node": "./src/utils/ckb-mmr-wasm/dist/node/node",
			"@/storage/load-driver": "./src/storage/drivers/node",
		}
	},
	async onSuccess() {
		// Copy WebAssembly files to dist directory
		const fullPath = (path: string) => new URL(path, import.meta.url).pathname

		let files = [{ from: "src/utils/ckb-mmr-wasm/dist/node/node_bg.wasm", to: "dist/node/node_bg.wasm" }]

		files = files.map((e) => ({
			from: fullPath(e.from),
			to: fullPath(e.to),
		}))

		// Ensure the destination directory exists
		for (const entry of files) {
			const dest_dir = dirname(entry.to)

			if (!existsSync(dest_dir)) {
				mkdirSync(dest_dir, { recursive: true })
			}

			// Copy the file
			copyFileSync(entry.from, entry.to)
		}

		console.log("📦 Copied WebAssembly files to dist directory")
	},
})
