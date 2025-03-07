import { defineConfig } from "vitest/config"
import tsconfigPaths from "vite-tsconfig-paths"
import { loadEnv } from "vite"
export default defineConfig({
    plugins: [tsconfigPaths()],
    test: {
        globals: true,
        // mode defines what ".env.{mode}" file to choose if exists
        env: loadEnv("custom", process.cwd(), ""),
        setupFiles: ["./src/tests/setup.ts"],
        reporters: ["verbose"],
        environment: "jsdom",
        coverage: {
            provider: "v8",
            reporter: ["text", "json", "html"],
            exclude: ["node_modules/**", "test/**"],
        },
    },
})
