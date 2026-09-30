# 2026-09-05 — Drop the overloaded `quote` from the IntentGatewayV3 ABI so subql codegen compiles

The gateway ABI regenerated in PR #1207 carried both `quote` overloads inherited from `HyperApp`
(`quote(DispatchPost)` and `quote(DispatchGet)`). subql's codegen names the generated transaction
type after the function alone, so it emitted `Quote_tuple_Transaction` twice and `subql build`
failed with TS2300 in CI. The indexer never calls `quote`, so both entries are removed from
`IntentGatewayV3.abi.json`; the events and the functions the handlers read are unchanged.
Verified with the `ENV=local` codegen chain CI runs.

Files: `src/configs/abis/IntentGatewayV3.abi.json`, `docs/ai/ChangeLog.md`.
