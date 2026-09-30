# 2026-08-19 — Cross-lane orders skip cleanly instead of erroring "Shared cache is not initialized"

An operator running only Base saw `ERROR: [intent-filler]: Shared cache is not initialized` whenever an order destined to a chain their filler does not run scrolled past. The solver-selection cache is only ever populated for configured, non-watch-only chains (boot's `solverSelectionChains`), so any other destination fell through `handleNewOrder`'s cache check and was dropped with an error that reads like the filler is broken. The behavior (drop) was correct — there is no client, bundler or float for an unconfigured destination — the diagnosis was not, and it dates to #287.

`handleNewOrder` now checks the destination first: not a configured chain, or configured but watch-only → debug-level skip, cache untouched. The "Shared cache is not initialized" error survives for what it actually means now — a configured, filling destination with a genuinely absent entry, which is an initialization bug. Pinned by `order-destination.test.ts`: unconfigured / non-EVM / watch-only destinations never touch the cache, a filling destination proceeds to the allowlist, and the configured-but-uncached case still errors.

Files: `src/core/filler.ts`, `src/tests/core/order-destination.test.ts` (new).
