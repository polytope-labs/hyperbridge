# 2026-09-10 — Vault transfers are capital movements; pre-tracking shares have an opening basis

Ordinary vault-share transfers are recorded as `TRANSFER_IN` and `TRANSFER_OUT` ledger entries,
valued with `convertToAssets` at the event block. Transfer IDs append the LP address to the existing
chain/transaction/log key because both owners can be tracked. Deposit and withdrawal IDs and counters
retain their meaning; mint/burn transfers and self/zero transfers do not create capital movements.
The new nullable position fields hold transfer totals and opening shares/principal/block. Existing
rows interpret null amounts as zero. Reusing DEPOSIT/WITHDRAW for transfers was rejected because it
misrepresents the onchain event and inflates actual deposit/withdrawal counts.

Stress-audit follow-up: IDs and stored transaction hashes are lowercase. Completed transfer sides
are deduplicated before valuation RPC. The non-capital Transfer predicate is shared by the handler,
service and block decoder, and runs before timestamp reads. Capital handlers opt into rethrowing
the shared wrapper's decode errors; acknowledging those errors could silently lose principal even
when later share counts happen to reconcile. Missing arguments and duplicate/invalid RPC log indexes
also fail processing. Other handlers retain the wrapper's default behavior. See the
`flows/vault-accounting-stress-audit.md` case matrix and independent-model tests.

For a new position, establish an opening basis for shares held before its first eligible vault event.
The safe ethers provider reads end-of-handler-block state. Subtract all share movements at or after
the triggering log in that same block from the live balance, then value the remainder at that block's
exchange rate. Subtracting only the triggering event fails when more capital moves later in the block.
The configured HTTP RPC supplies the single block's logs because SubQuery's safe provider does not
support getLogs. Missing triggering logs, impossible balances, or required RPC failures abort event
processing instead of recording an event with an invented zero baseline. No unbounded history scans
are added to indexer handlers. Yield before this opening block is deliberately outside the new
position's measurement period; receipt-time lifetime yield requires historical reconstruction.

SubQuery's Ethereum indexer runs block handlers before log handlers. Defer an LP's daily snapshot if
that block contains its capital movements, even if their share deltas cancel. Failed RPCs and deferred
LPs leave the daily completion gate open; successful LP snapshots are deduplicated on retry. A persistent
ledger/onchain share mismatch instead withholds that LP's snapshot and logs the need for reconciliation.
It does not prevent other LPs or the independent vault aggregate from being published. Genuine losses
remain negative yield. Share equality is a consistency check, not proof that an old ledger has complete
capital history (untracked historical transfers can net to zero shares).

Do not silently reset existing positions to today's value: that would erase earned yield and hide
historical corruption. Existing incorrect ledger/position/snapshot history still needs the separate
audited repair. Previously published snapshots are not rewritten by this change; clients can continue
to display an old bad value until that repair is applied. Positions without any eligible vault event
are not newly discovered just from enabling delegation.

Rollout uses the existing additive schema migration: rebuild manifests, codegen and bundle; restart
the substrate schema leader first, then EVM writers after it is healthy. New fields are nullable and
enum values are additive. Regenerate the EVM manifests to install the Transfer subscription. No SQL
repair, restart or production deployment is performed by the code change itself.
