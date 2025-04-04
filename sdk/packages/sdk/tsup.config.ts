import { defineConfig } from "tsup"

export default defineConfig({
	entry: ["src/index.ts"],
	outDir: "dist",
	format: ["cjs", "esm"],
	dts: true,
	sourcemap: true,
	clean: true,
	splitting: false,
	treeshake: true,
})
