# 2026-10-23 — Simplex runs on Node 24.22.1, and requires a Node with the Node2Shell fix

CVE-2026-111497 (Node2Shell) is an out-of-bounds write in the Brotli decoder bundled with Node. Any
Brotli data the process decompresses can trigger it, and that includes `fetch()` responses with
`Content-Encoding: br`. Simplex fetches from RPCs, the orderbook, the indexer and bundlers. A malicious
or compromised endpoint could send a crafted response and run code in the solver.

Remote access does not reach the decoder. The embedded SSH server only negotiates `zlib@openssh.com`
compression, and `UiServer` reads request bodies as raw bytes without decoding them. The exposure is
the runtime.

## Runtimes

- The Docker image's builder and runtime stages are on `node:24.22.1` and `node:24.22.1-slim`.
- The desktop app stages Node 24.22.1 (`NODE_VERSION` in `simplex-desktop/scripts/stage-node.mjs`).
  The release is signed by the same key the script already pins.
- The desktop release workflow packages on Node 24.22.1, and the staging check expects `v24.22.1`.

## `engines.node`

`engines.node` is `^22.23.4 || ^24.22.1 || >=26.12.1`: the first patched release on each supported
line. Node 23 and 25 are end of life and never got the fix, so they are no longer in range. The
`node:sqlite` floor (22.16.0) is still the oldest version the store can run on. `engines` is only
advisory, so npm and pnpm warn on an older Node rather than refuse to install.

Electron has no release with the fix yet. The packaged solver runs on the staged Node, not
Electron's, so this does not change what the solver is exposed to.
