# 2026-09-14 — Batch solver inventory reads through Multicall3

Solver inventory's storage reads were one `eth_call` at a time. For each solver, one after another, it called
`getCode`, then `balanceOf` per supported token, then `balanceOf` and `convertToAssets` per vault. On Base (7 tokens,
2 vaults) that was 12 round trips per solver, so a 20-solver genesis burst took 240 sequential requests.

Reads are now batched across every solver a pass handles:
- **Genesis.** A block's genesis reads take two Multicall3 `aggregate3` calls, however many solvers are pending: one
  for every balance, one valuing every vault balance. Each solver's `getCode` goes out alongside the first, capped
  at `MAX_CONCURRENT_READS` in flight; those stay one request each, since SubQuery hands a mapping a non-batching
  client on HTTP.
- **Refresh page.** A page takes at most three calls: the reconciliations' balances, their valuations (drift
  included), and the revaluations' valuations.
- **Share Transfer.** A share Transfer's single valuation is still one direct call.

The new `utils/multicall.ts` `readContracts` falls back to individual concurrent calls where Multicall3 has no code,
which is the case on Polkadot Asset Hub and Paseo. It also settles each read on its own, so a revert fails only the
solver that needed it.

Failure granularity changed with it:
- **Genesis.** A failed read now skips its solver, and the rest of the batch is applied. Before, the first failure
  ended the step.
- **Refresh.** A failed solver is skipped and the offset still advances, so one permanently failing solver no longer
  starves every page behind it. A failed batch still keeps the offset.

Files: `src/utils/multicall.ts`, `src/utils/__tests__/multicall.test.ts`, `src/configs/abis/Multicall3.abi.json`,
`src/services/solverInventory.service.ts`, `src/services/__tests__/solverInventory.service.test.ts`,
`docs/ai/flows/solver-inventory-discovery-genesis-transfers.md`,
`docs/ai/decisions/2026-09-14-solver-inventory-reads-are-batched-through-multicall3.md`
