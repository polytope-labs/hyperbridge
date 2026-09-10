# 2026-09-09 — The sandbox regression test runs the real bundle inside vm2, not a mock of the sandbox

Chosen: `phantom-decode.sandbox.test.ts` rolls the shipped `intents-helpers` entry into one file with esbuild
(resolved through `@subql/cli`, which is how the indexer itself is built) and executes the decoder inside a
`NodeVM` from the same vm2 `@subql/node-core` uses, with the host's `TextEncoder`/`TextDecoder` injected into
the sandbox. The live 11-byte declaration that was being dropped is the fixture.

Why the real thing. The failure is not "TextDecoder is undefined" — injecting it does not help — but that a
`Uint8Array` built inside the sandbox reaches host code as a proxy `ArrayBuffer.isView` rejects. Nothing short
of vm2 reproduces that: Node's own `vm` contexts hand over real typed arrays and decode fine, so a test built
on `vm.createContext` would have passed on the broken code. Injecting the codecs is the charitable setup — the
pre-fix bundle still threw the exact production error under it, and viem cannot load without `TextEncoder`.

Why a single-file bundle. vm2's own loader cannot walk the pnpm module graph: it does not follow the
`.pnpm` symlinks, refuses the dynamic `import()` `@polkadot/x-fetch` runs at load, and cannot parse ESM-only
packages such as `lodash-es`. The deployed indexer never hits any of this because `subql build` webpacks it
into one file; esbuild does the same for the test.

Not chosen — keeping the sandbox concern out of the SDK and injecting a VM2-safe declaration decoder from the
indexer like the fill and signature helpers. Those exist because viem is unusable in the sandbox and the SDK
must keep viem for every other consumer; the declaration codec needs nothing from any library, so making it
pure fixes the SDK for every consumer and leaves nothing to inject or drift.
