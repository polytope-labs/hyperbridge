# 2026-09-19 — Keep TronWeb out of the `intents-helpers` entry

`@hyperbridge/sdk/intents-helpers` exists so the SubQuery indexer can use the intents encoding helpers
without the full SDK bundle, which includes TronWeb. It had stopped doing that, and the indexer crashed
on every block it imported:

```
TypeError: 'deleteProperty' on proxy: trap returned falsish for property 'DEBUG'
    at t.save (debug/src/node.js:209:17)
    at 44142 (https-proxy-agent/dist/agent.js:19:32)
```

`tronweb` pulls `axios`, which pulls `https-proxy-agent`, which loads `debug`. `debug` runs
`delete process.env.DEBUG` as it initialises, and the indexer runs mappings in a VM2 sandbox that exposes
`process` through `freeze({ env })` — so the delete throws and the worker dies.

Two things put TronWeb in that entry, and both had to go:

- **`configs/chain.ts` called `TronWeb.address.toHex`** to write the Tron contract addresses. A Tron address is
  base58check over `0x41 || 20 address bytes || a 4-byte checksum`, so it now decodes the 20 bytes directly with
  `base58Decode` from `@polkadot/util-crypto`, which the SDK already depends on. Verified byte-identical against
  `TronWeb.address.toHex` for all six addresses in the file, and it now throws on a value that is not a 25-byte
  `0x41`-prefixed payload rather than returning a silently wrong address.
- **Rollup kept a bare `import "tronweb"`** in every entry of the node build even after tree-shaking every
  binding, because an external import is assumed to have side effects. `tsup-node.config.ts` now passes
  `treeshake: { moduleSideEffects: (id) => id !== "tronweb" }` so an entry using none of it drops the import.

The full `@hyperbridge/sdk` entry still bundles TronWeb — it uses it — and Tron support is unchanged. What changed
is that `intents-helpers` no longer loads it, and the indexer bundle now contains no `tronweb`, `axios`,
`https-proxy-agent` or `delete process.env.DEBUG`.
