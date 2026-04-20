import { defineConfig } from "vitest/config"
import tsconfigPaths from "vite-tsconfig-paths"

/** Vite warns when dependency sourcemaps reference files not shipped on npm (e.g. @uniswap/*). Vitest replaces customLogger, so we patch the resolved logger in a post plugin. */
const SOURCEMAP_MISSING_RE =
	/^Sourcemap for ".*" points to missing source files$/

export default defineConfig({
	plugins: [
		tsconfigPaths(),
		{
			name: "silence-sourcemap-missing-warnings",
			enforce: "post",
			configResolved(config) {
				const logger = config.logger
				const origWarnOnce = logger.warnOnce.bind(logger)
				logger.warnOnce = (msg, opts) => {
					if (SOURCEMAP_MISSING_RE.test(msg)) return
					origWarnOnce(msg, opts)
				}
			},
		},
	],
	test: {
		globals: true,
		setupFiles: ["./src/tests/setup.ts"],
	},
})
