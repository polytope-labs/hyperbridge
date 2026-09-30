# 2026-08-18 — A config with `simplex.signer` and no `signer` argument is a hard error

Chosen: `Simplex.start` throws when `config.simplex.signer` is set and `options.signer` is not.

Alternatives considered: (a) resolve the block automatically, keeping the old behaviour as a fallback; (b) ignore it silently, since the config block now belongs to the binary.

Why the throw won: (a) leaves two ways to choose the key that owns the solver's funds, one of them invisible in the `Simplex.start` call, which is exactly the coupling this change removes — and it would drag `@turnkey/sdk-server` and the MPCVault gRPC client into the boot path of every consumer whether or not they use them. (b) starts a solver on an address the operator did not intend — a config that names a key is evidence of intent, and quietly filling with a throwaway watch-only key instead is worse than not starting. The error names the fix (`signer: await createSigner(config.simplex.signer)`), so the TOML path stays one line away.
