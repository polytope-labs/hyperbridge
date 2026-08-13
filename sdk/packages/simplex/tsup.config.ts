import { defineConfig } from "tsup"

/** Native and worker-spawning modules that must never be inlined into a bundle. */
const ALWAYS_EXTERNAL = ["better-sqlite3", "@solana/spl-token", "@solana/web3.js", "pino", "pino-pretty", "thread-stream"]

/**
 * Dependencies inlined into the library build despite the general rule.
 *
 * The Uniswap SDKs publish ESM with extensionless relative imports
 * (`./addresses`, not `./addresses.js`), which esbuild resolves but Node's own
 * ESM loader refuses — so leaving them external makes `import
 * "@hyperbridge/simplex"` throw ERR_MODULE_NOT_FOUND in any plain Node app.
 * Inlining is safe here because neither type reaches the public API: they are
 * used inside the funding planners, so no consumer can hold an instance that
 * has to match ours.
 *
 * `ethers` rides along for the same reason — the v4 SDK reaches into
 * `ethers/lib/utils`, a v5 CJS subpath with no ESM-resolvable extension.
 */
const BUNDLE_INTO_LIBRARY = [/^@uniswap\//, /^ethers($|\/)/]

const ESM_REQUIRE_SHIM = {
	js: "import { createRequire } from 'module'; const require = createRequire(import.meta.url);",
}

export default defineConfig([
	/**
	 * Library entry points. Dependencies stay external and are resolved from the
	 * consumer's node_modules — bundling them would ship a second copy of viem,
	 * decimal.js and @hyperbridge/sdk alongside the app's own, and `instanceof
	 * Decimal` across that boundary is false. The CLI below is bundled precisely
	 * because it has no such boundary.
	 */
	{
		entry: { index: "src/index.ts", sqlite: "src/data/sqlite/index.ts" },
		format: ["esm", "cjs"],
		dts: true,
		splitting: false,
		sourcemap: true,
		clean: true,
		shims: true,
		noExternal: BUNDLE_INTO_LIBRARY,
		// tsup externalizes `dependencies` and `peerDependencies` automatically but
		// not `optionalDependencies`, so better-sqlite3 has to be named here or its
		// loader gets inlined and hunts for the native binding relative to dist/.
		external: ALWAYS_EXTERNAL,
		esbuildOptions(options, context) {
			if (context.format === "esm") options.banner = ESM_REQUIRE_SHIM
		},
	},

	/**
	 * The `simplex` binary: one self-contained file, so a global install or the
	 * docker image runs without resolving a dependency tree. tsup carries the
	 * shebang over from src/bin/simplex.ts and marks the output executable, so
	 * nothing has to patch it afterwards.
	 */
	{
		// Keyed, not a bare path: with one entry tsup would flatten the output to
		// dist/simplex.js and break the `bin` mapping.
		entry: { "bin/simplex": "src/bin/simplex.ts" },
		format: ["esm", "cjs"],
		dts: false,
		splitting: false,
		// No sourcemap: mapping a 15 MB single-file bundle costs ~29 MB per format
		// and 4× the published tarball, for a stack trace nobody debugging their
		// own app will read. The library entries keep theirs.
		sourcemap: false,
		clean: false,
		shims: true,
		noExternal: [/.*/],
		esbuildOptions(options, context) {
			options.external = ALWAYS_EXTERNAL
			if (context.format === "esm") options.banner = ESM_REQUIRE_SHIM
		},
	},
])
