# 2026-09-10 — Exclude transferred vault capital from LP yield

Subscribe to ordinary vault-share transfers and account for both tracked owners using event-block
asset values, with separate transfer totals and replay-safe ledger keys. Initialize preexisting shares
as opening principal when a position is first tracked, including correct handling of additional events
in the same block. Required accounting RPC failures propagate instead of silently dropping events.

Daily LP snapshots defer until the current block's capital events are indexed and withhold yield for
unreconciled share balances. Transient snapshot failures remain retryable without rewriting completed
LP snapshots. Existing historical corruption is left for an explicit repair. Tests cover the reported
Base USDC quantities, pre-delegation balances, multiple same-block events, transfers between tracked
LPs, mint/burn exclusion, replay, losses and RPC failures.

Files: `src/services/yieldVault.service.ts`, `src/utils/vaultAccounting.ts`,
`src/handlers/events/yieldVault/transfer.event.handler.ts`, `src/mappings/mappingHandlers.ts`,
`src/configs/abis/Erc4626.abi.json`, `src/configs/schema.graphql`,
`scripts/templates/evm-chain.yaml.hbs`, `src/services/__tests__/yieldVault.service.test.ts`,
`src/utils/__tests__/vaultAccounting.test.ts`,
`docs/ai/decisions/2026-09-10-vault-transfer-principal.md`,
`docs/ai/flows/pool-liquidity-refresh-orderfilled-partialfill-escrowreleased.md`.
