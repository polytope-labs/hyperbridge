# Review fixes: `patch` rejecting, duplicate candidate paths, stale API docs

Three findings from @Wizdave97 on #1234, all confirmed in the code before being acted on.

`SqliteStateStore.patch` guarded its write with `persist` but then called `read()` outside the
guard, so an unreadable `bids.db` still rejected. That is the only path production takes —
`Simplex.pause/resume`, the CLI's `setPaused` and the phantom batch all reach the store through
`patchRuntimeState`, and nothing outside tests calls `set`. `UiServer` pauses the filler at
`/api/pause` and *then* awaits `setPaused`, so the rejection returned a 500 while the filler was in
fact paused — exactly what `persist` promises will not happen. The write and the read-back now run
inside one guard, and a failure returns the requested patch: with the database unreadable that is
the most that can honestly be said, the failure is already logged, and no caller reads the value.
`persist` is generic over its return type to allow this. The existing test missed it because it
exercised `set`, which never reads.

The two candidate paths for the retired JSON file were not de-duplicated. `--data-dir .filler-data`
makes `join(dataDir, …)` and `resolve(cwd, ".filler-data", …)` the same file under a relative and
an absolute string, so it was imported twice and unlinked twice — the second failing with ENOENT
and logging that a stale file had survived when it had not. Both paths are now `resolve`d and
passed through a `Set`.

`docs/content/developers/sdk/api/simplex.mdx` still described `SqliteDataStore` as keeping
`runtime-state.json`, and its `StateStore` snippet omitted the new optional `patch`. Since the
stated reason for making `patch` optional is third-party implementers, and that page is what they
read, leaving it out would have shipped them the read-merge-write race. `RuntimeState` was also
still documented as `{ paused?: boolean }`, from before `phantomBids` existed.

Two tests, each failing against the previous behaviour: `patch` on a closed database resolves
rather than rejecting, and a data directory that *is* the legacy directory imports once with no
"could not delete" warning. The second reads the store's log through a `LoggerContext` sink.

Files: `src/data/sqlite/state.ts`, `src/tests/data/state-store.test.ts`,
`docs/content/developers/sdk/api/simplex.mdx`, `docs/ai/flows/operator-state-on-disk.md`.
