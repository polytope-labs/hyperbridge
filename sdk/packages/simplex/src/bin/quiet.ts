/**
 * Console hygiene for the BINARY, applied before anything else loads — this
 * module must stay the entry's first import, because most of the noise fires
 * during `@polkadot/*` module initialisation.
 *
 * Bin-only on purpose. The library never touches the host's console or
 * process: a library consumer seeing duplicate-package warnings has a real
 * dedupe to do in their own tree, and hiding it from them is not our call. The
 * bundled CLI is different — its single-file build necessarily instantiates
 * the ESM and CJS halves of the `@polkadot/*` interop side by side, so the
 * duplicate-package detector fires on every start about a duplication the
 * operator can do nothing about.
 */

// polkadot's official escape hatch: silences the detector for SAME-version
// duplicates (the bundle's dual instantiation). Read at each package's module
// init, hence the first-import requirement.
process.env.POLKADOTJS_DISABLE_ESM_CJS_WARNING = "1"

// Node's deprecation chatter (punycode, via transitive deps). Deprecations
// only — every other process warning still prints.
;(process as { noDeprecation?: boolean }).noDeprecation = true

/**
 * Known-noise patterns from `@polkadot/*` init and its logger. Deliberately
 * narrow: a pattern here must identify chatter that carries no operator
 * action, not merely mention polkadot.
 *
 * - The multiple-versions detector, for the case the env flag does not cover
 *   (genuinely different versions in the tree — a packaging fact of the
 *   bundle, actionable only at build time, never at run time).
 * - `REGISTRY: Unknown signed extensions` — Hyperbridge declares extensions
 *   this @polkadot/api version does not know; it already treats them as
 *   no-effect, which is correct.
 * - `API/INIT: RPC methods not decorated` — the node exposes namespaces
 *   (ismp_*, intents_*) the api has no decorators for; we call them via
 *   raw RPC, so the gap is by design.
 *
 * Exported for tests.
 */
export const POLKADOT_NOISE = [
	/@polkadot\/\S+ has multiple versions/,
	/@polkadot\/\S+ requires direct dependencies exactly matching/,
	/REGISTRY: Unknown signed extensions/,
	/API\/INIT: RPC methods not decorated/,
]

/** Whether one console call is known polkadot chatter. Exported for tests. */
export function isPolkadotNoise(args: readonly unknown[]): boolean {
	const text = args.map((a) => (typeof a === "string" ? a : "")).join(" ")
	return POLKADOT_NOISE.some((pattern) => pattern.test(text))
}

/** The filter around a warn sink. Exported so tests can observe it directly. */
export function filteringWarn(sink: (...args: unknown[]) => void): (...args: unknown[]) => void {
	return (...args: unknown[]) => {
		if (!isPolkadotNoise(args)) sink(...args)
	}
}

// Only `console.warn` is patched: both the duplicate-package detector and the
// polkadot logger's warn level write there, and nothing else does. Errors are
// left alone entirely — a filter has no business near those.
console.warn = filteringWarn(console.warn.bind(console))
