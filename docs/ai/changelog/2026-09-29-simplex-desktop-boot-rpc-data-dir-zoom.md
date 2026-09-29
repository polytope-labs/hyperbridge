# Simplex desktop: boot with an unreachable RPC, data directory name, zoom

## A dead first RPC no longer stops startup

`OrderScanner.create` read each chain's id from `rpcUrls[0]` alone. If that one endpoint could not
answer, the whole filler failed to start, even when every other endpoint was healthy. The desktop
wizard showed this as `fetch failed` after saving the config.

This happened with the default Ethereum list: its first entry, `https://mainnet.gateway.tenderly.co`,
returns NXDOMAIN on Quad9 (`9.9.9.9`).

The scanner now resolves a missing `chainId` with `resolveChainConfigs(..., { tolerateUnreachable: true })`,
the same rule boot already used. Endpoints that cannot answer are logged and kept for the quorum
client to judge. Startup fails only when no endpoint answers or the endpoints disagree on the chain.
Runtime endpoint edits still pass a strictly checked `chainId`, so they are unchanged.

## Data directory is named `Simplex`

Electron names `userData` after `productName` in `package.json`. `simplex-desktop/package.json` had
none, so data lived under `@hyperbridge/simplex-desktop`, for example
`~/Library/Application Support/@hyperbridge/simplex-desktop`. It now sets `"productName": "Simplex"`,
matching `electron-builder.yml` and the paths in the docs. A workspace-policy test keeps the two
names equal. Existing pre-release installs start fresh in the new directory.

## Zoom

The application menu has a View menu with Actual Size, Zoom In (`Cmd/Ctrl+=` and `Cmd/Ctrl+Plus`),
Zoom Out, and Toggle Full Screen. Reload and developer tools are intentionally left out.
