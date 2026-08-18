# ChangeLog

AI-maintained log of code changes in `sdk/packages/simplex`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/simplex/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-18 — `Simplex.start` takes a Signer interface instead of a signer config

Signing was reachable only through `config.simplex.signer`, a tagged union over the three backends simplex happens to ship, so a consumer embedding the library could not sign with anything else. `Simplex.start` now takes `signer: Signer` — the interface the solver already used internally — and the config block is the binary's TOML format only.

The interface was cut to what the solver actually calls: `account`, `signTypedData` and `signRawHash`, plus optional `mode` (a free-form log label, was a closed union of the three built-ins) and `signAuthorization`. `signMessage` went, along with its declaration in the SDK's `SigningAccount` — bids are signed as EIP-712 UserOperations, and no path in either package ever invoked it. `sendEip7702DelegationTransaction` went too: `DelegationService` now always sends through the signer's wallet client, and MPCVault's inability to carry an authorization list in its structured send is handled inside the viem account `createMpcVaultAccount` builds, which serialises and raw-signs set-code transactions.

Added `viemSigner(account)`, which derives the whole interface from any viem local account, and rebuilt `privateKeySigner` and `turnkeySigner` on top of it (Turnkey keeps its structured `signAuthorization`). Factories renamed to their public names: `createSimplexSigner` → `createSigner`, `create*SigningAccount` → `privateKeySigner` / `mpcVaultSigner` / `turnkeySigner`, `initializeSignerFromToml` → `signerFromToml`, and the `SigningAccount` type → `Signer`.

`FillerTomlConfig` (exported as `SimplexConfig`) no longer declares `simplex.signer` at all. The block moved to a new `FillerConfigFile extends FillerTomlConfig`, the on-disk shape the binary parses, which the CLI, the setup API, the TOML writer, the wizard and the dashboard's config type now use. The binary passes the parsed file straight to `Simplex.start`, extra key and all — `UiServer.persistConfig` regenerates the TOML from the running config object, so stripping the block there would erase the operator's signer from their file on the next curve edit.

Boot now rejects a missing signer unless every chain is watch-only, replacing the `[simplex.signer]` presence check inside `validateConfig` (a config-shape validator has no business requiring an argument that is no longer part of the config; it still validates the block when one is present). `Simplex.start` throws when an object carries `simplex.signer` and no `signer` was passed — a runtime check, since the type no longer has the field — rather than silently ignoring it or resolving it. The CLI resolves the TOML block into a `Signer` itself and keeps its file-oriented error message.

Files: `sdk/packages/sdk/src/types/index.ts` (SigningAccount narrowed), `src/services/wallet/types.ts`, `src/services/wallet/mpcvault.ts`, `src/services/wallet/signer.ts`, `src/services/wallet/index.ts`, `src/services/wallet/accounts/viem.ts` (new), `src/services/wallet/accounts/privatekey.ts`, `src/services/wallet/accounts/mpc.ts`, `src/services/wallet/accounts/turnkey.ts`, `src/services/DelegationService.ts`, `src/services/ChainClientManager.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/index.ts`, `src/bin/simplex.ts`, `src/config/filler-toml.ts`, `src/services/server/{UiServer,setup-api}.ts`, `src/cli/init/{index,state,emit-toml}.ts`, `src/cli/init/steps/write.ts`, `ui/src/types.ts`, plus type-only renames across `src/core/filler.ts`, `src/services/{ContractInteractionService,PaymasterKeeperService,UserOpSender}.ts`, `src/strategies/fx.ts`. Tests: `src/tests/wallet/signer.test.ts` (new), `src/tests/wallet/turnkey.test.ts`, `src/tests/wallet/mpcvault.integration.test.ts`, `src/tests/cli/filler-toml-validate.test.ts`. Docs: `README.md`, `docs/content/developers/sdk/simplex.mdx`, `docs/content/developers/sdk/api/simplex.mdx`, `docs/content/developers/evm/intent-gateway/simplex.mdx`.
