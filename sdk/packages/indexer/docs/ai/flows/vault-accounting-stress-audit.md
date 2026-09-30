# Vault accounting stress audit

Verified 2026-09-10 against the vault service, four vault handlers, ABI/log reader, generated models,
manifest template, and Simplex's `TokenSender` / `VaultFundingPlanner` call construction. The focused
suite has 99 passing cases, including 25 deterministic seeds with 120 generated actions each (3,000
actions, excluding setup), and a 205-LP paging case. It exercises 100% of executable lines/statements
and functions in the vault service, handlers and accounting helpers; branch coverage across those
files and the shared event wrapper is 97.27%, with every handler branch covered.

The stress harness feeds ABI-encoded logs through the real deposit, withdrawal and transfer handlers,
the real block-log decoder and generated store models. An independent model maintains wallet share
balances and a single signed capital-flow total, while RPC returns end-of-block state. Each snapshot
is checked against `value(shares) - net capital`. RPC/store boundaries and inventory publication are
mocked. Reorg and failed-block cases explicitly simulate the node's store rollback before replay;
these are not live-node or fork-chain tests. No transactions or database repairs were sent.

## Simplex sends and receives

`TokenSender.send` redeems configured share tokens with `redeem(shares, recipient, solver)`. The
recipient gets underlying assets. For an underlying-token send with insufficient working balance,
it batches `withdraw(amount, solver, solver)` and an ERC-20 transfer. A later vault sweep calls
`deposit(amount, solver)`. A share transfer from another wallet or tool is a separate ERC-20 Transfer
on the vault contract and needs transfer principal accounting.

| Scenario | Expected accounting and test result |
| --- | --- |
| Deposit and mint into an empty tracked wallet | Deposit events alone add emitted assets and minted shares; mint Transfer logs add nothing. Pass. |
| Partial withdraw, full redeem, then deposit again | Withdraw owner loses shares; withdrawn assets reduce net capital. Previously earned yield survives an empty position and later deposits. Pass. |
| Simplex redeems shares directly to an empty recipient | Only the solver's withdrawal is recorded. Recipient owns underlying, not shares, and has no vault position. Pass. |
| Simplex redeems to a recipient with its own vault shares | Recipient's existing principal/shares are unchanged; its wallet token balance cannot enter vault yield. Pass. |
| Receiver subsequently sweeps underlying into a vault | Exactly one new Deposit adds principal for the receiver. Pass. |
| Simplex withdraws a shortfall and sends underlying in one batch | One withdrawal for the solver; underlying transfer adds no vault ledger entry. Pass. |
| Raw underlying sends/receipts and paymaster token charges | No change to vault principal. Pass. |
| Direct share transfer to an empty tracked recipient | Recipient gets transfer principal at the event-block share value; sender retains previously earned yield. Pass. |
| Direct share transfer to an existing recipient | Transfer totals extend its existing deposit/withdrawal history without overwriting counters or principal. Pass. |
| Sender/receiver eligibility combinations | Both, only sender, only receiver, and neither tracked are covered. Only eligible sides get ledger entries. Pass. |
| Shares received before delegation | First eligible vault event establishes an opening basis for shares already present. Pre-baseline appreciation is not reported as newly earned yield. Pass. |
| Delegation revoked after tracking began | Existing position continues to record outgoing transfers and redemptions. Pass. |
| Caller, share owner and withdrawal recipient differ | Deposits belong to Deposit.owner; withdrawals to Withdraw.owner. Pass. |
| Multiple transactions, deposits, withdrawals and share round trips in one block | Opening shares exclude all remaining block movements; subsequent events are counted once. Pass. |
| Snapshot before a zero-net-share round trip is indexed | Defer until event principal is folded; share equality alone is not enough. Pass. |
| Share decimals differ from underlying decimals | Six- and eighteen-decimal share cases use raw integers and convertToAssets; no assumed 1:1 conversion. Pass. |
| Dust, floor/ceil rounding and falling exchange rates | Zero-asset transfers still move nonzero shares; genuine rounding losses and vault losses remain signed. Pass. |
| Duplicate event delivery, including different hex casing | No repeated principal or share adjustments. Pass. |
| Self, mint, burn and zero-share transfers | Excluded before timestamp RPC; no capital movement. Pass. |
| Unknown vault/chain or unrelated delegation | No invented position. Pass. |
| RPC failures, incomplete logs and duplicate RPC log indexes | Required accounting failures propagate; no invented opening baseline. Snapshot failures remain retryable. Pass. |
| Store failure between the two transfer sides | Error propagates; simulated block rollback/replay applies both sides once. Pass. |
| Reorg replaces a transfer's recipient | Simulated historical rollback removes the orphaned recipient and replay produces the canonical balances. Pass. |
| 205 LPs, multiple vaults/chains, day rollover | All pages are covered, positions stay isolated, and completed daily snapshots are not overwritten. Pass. |
| Legacy position with unexplained share drift | Withhold that LP's new yield, while healthy LPs and vault aggregate continue. Pass. |

## Findings fixed during this audit

- The generic event wrapper acknowledged decoding failures after logging them. Capital handlers now
  opt into rethrowing decode failures and reject missing arguments, so these movements cannot be
  silently skipped. The default behavior for other handlers is unchanged and tested.
- Transaction hash casing previously changed ledger IDs and allowed duplicate folding. IDs and stored
  hashes are canonicalized to lowercase. Transfer replay checks run before valuation RPC.
- Excluded Transfer events previously reached timestamp/RPC processing before the service filtered
  them. A shared predicate now filters in the handler, service and block-log decoder.
- Duplicate RPC log indexes previously doubled movements used to infer opening capital. The reader
  now rejects duplicate or invalid indexes.

## Remaining limits

Existing incorrect historical rows still need the audited repair. Equal historical share counts do
not prove complete historical cash flows, since missed incoming and outgoing transfers can cancel.
Newly tracked opening capital is valued at the first eligible event's block; earlier lifetime yield
requires reconstructing earlier receipts. Enabling delegation without any later eligible vault
event does not itself create a vault position. Current-block share transfers use the block's exchange
rate; the tests do not claim transaction-intermediate pricing when a vault rate changes inside a
block. These are vault-yield figures, not whole-wallet trading P&L or gas-adjusted returns.

## Reproduce

From `sdk/packages/indexer`, after normal codegen:

```sh
../../node_modules/.bin/jest --runInBand --silent \
  src/services/__tests__/yieldVault.service.test.ts \
  src/services/__tests__/yieldVault.stress.test.ts \
  src/utils/__tests__/vaultAccounting.test.ts \
  --coverage \
  --collectCoverageFrom='src/services/yieldVault.service.ts' \
  --collectCoverageFrom='src/handlers/events/yieldVault/*.ts' \
  --collectCoverageFrom='src/utils/vaultAccounting.ts' \
  --collectCoverageFrom='src/utils/event.utils.ts'
```
