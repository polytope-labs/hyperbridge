import { defineConfig } from "tsup"
import { copyFileSync, mkdirSync, existsSync } from "fs"
import { join, dirname } from "path"

export default defineConfig({
	entry: ["src/index.ts"],
	outDir: "dist",
	format: ["cjs", "esm"],
	dts: true,
	sourcemap: true,
	clean: true,
	splitting: false,
	treeshake: true,
	async onSuccess() {
		// Copy WebAssembly files to dist directory
		const wasmSourcePath = join(__dirname, "src", "utils", "ckb-mmr-wasm", "ckb_mmr_wasm_bg.wasm")
		const wasmDestPath = join(__dirname, "dist", "ckb_mmr_wasm_bg.wasm")

		// Ensure the destination directory exists
		const destDir = dirname(wasmDestPath)
		if (!existsSync(destDir)) {
			mkdirSync(destDir, { recursive: true })
		}

		// Copy the file
		copyFileSync(wasmSourcePath, wasmDestPath)
		console.log("Copied WebAssembly file to dist directory")
	},
})
