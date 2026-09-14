import { defineConfig } from "tsup"

export default defineConfig({
	entry: { main: "src/main.ts" },
	format: ["esm"],
	platform: "node",
	target: "node24",
	bundle: true,
	clean: true,
	sourcemap: true,
	external: ["electron"],
})
