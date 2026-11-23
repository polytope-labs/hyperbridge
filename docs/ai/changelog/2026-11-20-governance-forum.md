# Governance forum on omniplex

Hyperbridge runs its own governance forum: the SubSquare frontend fork, served by our API in
`governance/api`, reading Nexus governance indexed by omniplex.

## substrate-eth-rpc

The `governance` key gives an `Event` log, the explorer's signature, for each event of
`Referenda`, `ConvictionVoting`, `Scheduler`, `TechnicalCollective`, `Whitelist`, `Preimage`
and `Treasury`, with arguments as JSON and accounts in SS58 with the chain's prefix. A node
always serves them.

Four reads on the storage address decode with the runtime that wrote the block's state:
`getStorageJson` (a storage value, typed by the entry its key belongs to), `getConstantJson`,
`decodeCallJson` and `getExtrinsicCallJson` (the call in one of the block's extrinsics whose
encoding hashes to a given hash, for a call storage never kept past the block). Each answers an
empty string when there is nothing to decode. Anyone
who reaches the RPC can call them, so their input is checked before anything is allocated:
declared lengths must fit the bytes given, nesting is capped at 48 levels (about 24 nested
calls, which keeps the JSON parseable), trailing bytes are rejected and calls are limited to
4 MiB. Decoding runs on the blocking pool under the upstream concurrency permit. A read of a
block in the cache makes no upstream call, and metadata is fetched once per runtime.

## Indexing

A config with `mode = "governance"` (`omniplex/config/governance-mainnet.toml`) runs only the
governance handlers, from block 5986840 where OpenGov was added. The indexer and a governance
config both write the Nexus metadata table, so each records its mode in the database and
refuses one the other has used; the explorer keeps no metadata and can share either's. Its RPCs must be an archive node
whose substrate-eth-rpc includes the reads above.

The handlers write the `governance` schema (migration `0004_governance`) with the tables and
columns the API reads:

- Referenda with their tally, electorate (total minus inactive issuance), live details kept
  once they end, track definition at the last state change, decoded call and enactment.
  Deposit refunds and metadata events stay in the history with `changes_state = false` and
  never change a referendum's state.
- Votes and delegations, re-read from `ConvictionVoting::VotingFor` on each voting event,
  since Nexus vote events carry no referendum index. A vote or removal reads only the tracks
  of ongoing referenda; delegation changes read every track. Votes on finished referenda keep
  their final state.
- Technical committee motions. Their votes and deadline come from `Voting` while the motion is
  open, which also drops the votes of members who leave the committee, and from each `Voted`
  event when it closed in the same block. Their call comes from `ProposalOf`, or, for a motion
  closed in the block it was proposed in, from the call in the proposing extrinsic that hashes
  to the motion's hash. Storage keeps neither past a close, and such a motion has no deadline.
- Tally snapshots, vote calls, whitelisted calls, preimages (bytes kept after they are cleared,
  and by a noting that is gone by the end of its block) and treasury spends.

At cutover, `import-subquery` on a copy of SubQuery's governance database keeps each row's
current version and continues after its last indexed block, so history is not re-read. It sets
`changes_state` on every referendum event, which SubQuery left empty on older rows.

## API

`governance-api` serves the forum frontend on `PORT` (7071) from a connection pool with a
per-statement timeout, and keeps the forum's own content in the `forum` schema, whose
statements are idempotent so a restart upgrades it. It starts before the indexer has made
progress and reads the indexed height through `forum._metadata`.

| Setting | Meaning |
|---------|---------|
| `DATABASE_URL` | The governance database |
| `FORUM_DOMAIN` | The domain sign-in messages name; required |
| `ADMINS` | Addresses allowed to moderate |
| `CORS_ORIGINS` | Origins browsers may call it from |
| `TRUSTED_PROXIES` | Proxies whose `X-Forwarded-For` is believed; other visitors are keyed by their own address |

Sign-in is a readable message naming the forum's domain, a nonce and its issue time, signed by
the wallet. Sessions are stored as hashes. Signed edits name the forum's domain, which the
frontend adds to each signed request, and are single use within a ten-minute window. Addresses are canonicalised to SS58 prefix 0 wherever they enter. Uploads have a
per-account quota and must be images by their content; served files carry `nosniff` and a
sandbox policy. `import <site>` copies the current forum's content and keeps going past a
missing file, and `verify <site>` compares this API with the current forum.

The compose file runs the indexer and the API under the `governance-mainnet` profile, with the
API bound to localhost behind the site's proxy. The API's integration test applies migration
`0004_governance` and runs in CI with the omniplex tests.
