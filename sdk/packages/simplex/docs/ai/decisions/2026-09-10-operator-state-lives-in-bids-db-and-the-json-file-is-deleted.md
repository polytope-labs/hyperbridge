# Operator state lives in `bids.db`, and `runtime-state.json` is deleted on sight

Chosen: `SqliteStateStore` keeps `RuntimeState` as one row per key in `bids.db`, the database the
bid store already owns. The JSON file it replaces is imported once, from the data directory and
from the older `.filler-data/`, and then unlinked from both.

The file was never a decision — it arrived with pause/resume in July, before `SimplexDataStore`
existed, and survived the library extraction unexamined. It had two defects. `writeFileSync` opens
with truncate, so a crash mid-write left a file that would not parse, and the reader's fallback
returned `{}`: a paused filler resumed and a live phantom bid was forgotten, which are the only two
things the file carried. And `patchRuntimeState` read, merged and wrote back, so the phantom batch
and an operator pause could drop one another's key — the merge helper narrowed that window but
could not close it. Both are free in SQLite, which the process already has open on this exact file.

Alternatives rejected: a temp-file-plus-rename would have made the write atomic but left the
read-modify-write race and a third storage format in a directory that already has two databases. A
separate `state.db` would add a file and a handle to carry one boolean. Keeping the JSON file
around after importing it was rejected too — an empty database reads it again, so a copy left
behind resurrects a pause the operator has since lifted. The cost is that a downgrade past this finds no
state and starts unpaused; the version that reads the file is the one that deletes it, so this is a
one-way door by construction.

`StateStore.patch` is optional rather than required, because the interface is public and a consumer
backing it with Postgres or an HTTP API should not have to implement two writers to keep compiling.
`patchRuntimeState` prefers it and falls back to get-then-set, which is what `MemoryDataStore` uses.
