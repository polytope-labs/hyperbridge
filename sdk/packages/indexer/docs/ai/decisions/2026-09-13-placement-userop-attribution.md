# 2026-09-13 — Placement user operation attribution

Store the placement operation as nullable `IOrderV3.userOpHash`, separate from the transaction hash and from fill operation hashes. Existing rows remain null until replayed; this change performs no backfill. It uses the existing placement save, without adding another parent-row writer or changing lifecycle updates.

`IntentGatewayV2.placeOrder` overwrites `order.user` with `msg.sender`. Match that placing account (converted from bytes32 to an address), rather than the output beneficiary, to the first canonical EntryPoint `UserOperationEvent` after the placement. A sender mismatch returns no hash, even if another matching operation appears later. Multiple placements inside an operation can share its hash; separate operations by the same account are distinguished by log order.

For placement, require the matching EntryPoint's latest `BeforeExecution` marker preceding the operation to also precede the placement. [EntryPoint v0.8](https://github.com/eth-infinitism/account-abstraction/blob/v0.8.0/contracts/core/EntryPoint.sol) validates all operations before that marker. This prevents a placement during validation, or before a later bundle, from being mislabeled with the next operation. Canonical v0.6/v0.7/v0.8 addresses share these event ABIs. Unknown EntryPoints and routes where the gateway caller differs from the operation sender remain null.

Rejected: deriving from the transaction sender (the bundler), selecting any later operation by the same account, or requiring a hash for every placement. Optional receipt lookup errors are logged and leave enrichment unset. Existing hashes are retained on replays whose lookup fails. The established fill matcher is extracted unchanged; its behavior is outside this placement addition.

## Migration verification

The checked-in `scripts/tests/verify-fill-schema-migration.cjs` migrates the main schema from the PR refresh to the expanded schema, verifies old rows and null defaults, inserts data into all seven new fields, and restarts with unchanged SDL. It creates a unique schema in a disposable database.

From the repository root:

```sh
git show 638ce716a:sdk/packages/indexer/src/configs/schema.graphql > /tmp/pr1112-placement-baseline.graphql
# Set MIGRATION_DATABASE_URL to a disposable database reachable from the container.
docker run --rm --entrypoint node \
  -e MIGRATION_DATABASE_URL -e BASELINE_SCHEMA=/baseline.graphql \
  -v /tmp/pr1112-placement-baseline.graphql:/baseline.graphql:ro \
  -v "$PWD/sdk/packages/indexer/src/configs/schema.graphql:/schema.graphql:ro" \
  -v "$PWD/sdk/packages/indexer/scripts/tests/verify-fill-schema-migration.cjs:/verify.cjs:ro" \
  polytopelabs/subql-node-substrate:v6.4.8-0 /verify.cjs
```
