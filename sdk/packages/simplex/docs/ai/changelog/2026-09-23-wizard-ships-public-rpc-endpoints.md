# 2026-09-23 — The wizards ship public RPC endpoints

Both setup wizards start each mainnet chain with a working set of public RPC endpoints, so the only
endpoint an operator has to supply is an ERC-4337 bundler. One Alchemy API key covers that for every
supported chain.

## What changed for the operator

Before, every chain needed an RPC URL typed or pasted in, and the wizard's own copy told the operator
that free endpoints would break event scanning. That was true of a single free endpoint. It is not
true of a quorum of them, which is what the filler has read through since quorum scanning landed.

The Alchemy key now fills in bundlers only. It no longer overwrites the first RPC field, because
pointing the scan at Alchemy would spend the key's quota on polling and put the chain back on a
single provider.

## The endpoint sets

`FREE_RPC_URLS` in `cli/init/chains.ts`, surfaced per chain as `InitChainMeta.defaultRpcUrls`. The
web wizard seeds `rpcUrls` from it; the CLI offers it as a default and skips the URL prompt and the
quorum follow-up when taken. Testnets have none — nothing was measured there.

| chain | endpoints | quorum bar |
|---|---|---|
| Ethereum | 9 | 7 |
| Base | 7 | 5 |
| Arbitrum | 7 | 5 |
| BNB Chain | 6 | 5 |
| Polygon | 5 | 4 |

Every URL scanned mainnet alone for 20 minutes, then ran 19 hours in a quorum without once being
benched. Endpoints benched chronically over that run are absent: `*.rpc.sentio.xyz` sat behind the
head for 8961 scans and so never cast a vote, and `thirdweb`, `blockmachine` and `swiftnodes` were
rate-limited in the hundreds.

## Two properties the lists have to keep

**One endpoint per operator.** `validateRpcUrls` rejects a repeated hostname, which does not catch
`gateway.tenderly.co/public/polygon` sitting beside `polygon.gateway.tenderly.co`. Two URLs from one
provider are one opinion — they agree with each other by construction, so counting both inflates the
denominator of a threshold that exists to survive a provider being wrong. Dropping those pairs is why
Polygon is five endpoints and not six.

**Room to lose one.** A BFT bar tolerates `floor((n-1)/3)` faults, and at `n = 3` that is zero: the
set becomes only as reliable as its flakiest member. Every chain keeps at least one endpoint of slack
over its bar.

`src/tests/cli/free-rpc-defaults.test.ts` pins both, along with chain coverage and the absence of API
keys. A keyed public endpoint would put every operator on one shared quota, and directory-published
keys rot.

The sets are a starting point, not a promise. Free endpoints come and go, both wizards leave every
field editable, and an operator with a provider account should use it.
