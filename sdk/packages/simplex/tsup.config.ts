import { defineConfig } from "tsup"

/** Native and worker-spawning modules that must never be inlined into a bundle. */
// ssh2 probes for an optional native crypto binding relative to its own package
// directory, so inlining it sends that lookup into dist/ instead.
const ALWAYS_EXTERNAL = ["@solana/spl-token", "@solana/web3.js", "pino", "pino-pretty", "thread-stream", "ssh2"]

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

/**
 * tsup 8 rewrites `node:foo` imports to bare `foo` by default
 * (`removeNodeProtocol`, flipped to false in tsup 9). For most builtins that is
 * harmless — `fs` and `path` resolve either way — but `node:sqlite` has no
 * unprefixed alias, so a stripped import is an ERR_MODULE_NOT_FOUND the moment
 * the binary starts. Nothing catches it in the test suite, which runs from
 * source where no rewriting happens; `scripts/build.sh` greps the bundle for it
 * instead.
 */
const KEEP_NODE_PROTOCOL = { removeNodeProtocol: false } as const

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
		...KEEP_NODE_PROTOCOL,
		dts: true,
		splitting: false,
		sourcemap: true,
		clean: true,
		shims: true,
		noExternal: BUNDLE_INTO_LIBRARY,
		// tsup externalizes `dependencies` and `peerDependencies` automatically;
		// the list above is for modules reached transitively, which it would
		// otherwise inline.
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
		...KEEP_NODE_PROTOCOL,
		// ESM only: `bin` and the docker ENTRYPOINT both execute simplex.js, so a
		// CJS twin was 15 MB of tarball nothing ever ran. Dropping it pays for the
		// map below.
		format: ["esm"],
		dts: false,
		splitting: false,
		// With the map, a crash names src files and lines. Without it, every report
		// reads `at simplex.js:328059:15` — an offset into a 15 MB bundle that
		// nobody, including us, can act on. The flag rides in the shebang
		// (`env -S node --enable-source-maps`) and the docker ENTRYPOINT — the
		// runtime API cannot map the very module it is called from, which a
		// single-file bundle is.
		sourcemap: true,
		clean: false,
		shims: true,
		noExternal: [/.*/],
		esbuildOptions(options, context) {
			options.external = ALWAYS_EXTERNAL
			if (context.format === "esm") options.banner = ESM_REQUIRE_SHIM
		},
	},
])
