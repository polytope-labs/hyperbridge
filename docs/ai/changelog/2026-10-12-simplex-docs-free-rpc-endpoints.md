# 2026-10-12 — Simplex docs document `[[chains]]` and the free RPC endpoints

The Simplex configuration page gains a `[[chains]]` section covering `rpcUrls`. Its "Free RPC
endpoints" subsection gives one `[[chains]]` block per mainnet chain. The uncommented URLs are
`FREE_RPC_URLS` in `sdk/packages/simplex/src/cli/init/chains.ts`, and the commented ones also passed
testing. The block parses with the `toml` package Simplex uses.

The Bundler section no longer mentions `bundlerUrl`. The overview no longer says free RPCs will not
work or that archive access is required. The installation, Confirmations and RPC, and
troubleshooting pages link to the new subsection.
