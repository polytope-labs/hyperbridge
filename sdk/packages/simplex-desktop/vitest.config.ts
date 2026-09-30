import { defineConfig } from "vitest/config"
import { fileURLToPath } from "node:url"

export default defineConfig({
	resolve: {
		alias: {
			"@hyperbridge/simplex/config-storage": fileURLToPath(
				new URL("../simplex/src/config/storage.ts", import.meta.url),
			),
		},
	},
	test: {
		include: ["src/tests/**/*.test.ts", "scripts/**/*.test.ts"],
	},
})
