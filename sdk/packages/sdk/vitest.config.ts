import { defineConfig } from "vitest/config"
import tsconfigPaths from "vite-tsconfig-paths"

export default defineConfig({
	plugins: [tsconfigPaths()],
	test: {
		globals: true,
		// mode defines what ".env.{mode}" file to choose if exists
		setupFiles: ["./src/tests/setup.ts"],
		reporters: ["verbose"],
		environment: "node",
		coverage: {
			provider: "v8",
			reporter: ["text", "json", "html"],
			exclude: ["node_modules/**", "test/**"],
		},
	},
})
