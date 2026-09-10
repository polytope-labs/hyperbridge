# 2026-09-10 — Review fixes: a kept launch file, and a token view that switched itself off

Two findings from review, both in code added on this branch.

**A stream error threw away history that was still readable.** The handler cleared `this.file`
alongside `this.stream` for every error, on the reasoning that "a half-owned file must not be read
back as this launch's history". That is true of the `wx` collision it was written for, where nothing
of this launch ever reached the path. It is not true of a failure part-way through a run: a full
disk at hour six leaves a file that is ours and complete up to that point, and clearing the path
made `stats()` report memory-only and short-circuited `recent()` at `if (!needsHistory || !this.file)`
— so the operator opening the page during a disk-full incident got the last 5000 records while the
six hours they came for sat readable beside them.

The discriminator is `stream.pending`, not an errno list: still pending means the open failed and
the path is not ours; already open means the file is. That also covers open-time `EACCES` and
`EISDIR` correctly, which an `EEXIST` check would have got wrong in the other direction — retaining
a path to a file that was never created. The footer now distinguishes the three states: recording,
recording-stopped-with-history-on-disk, and memory-only, instead of calling the middle one
"memory only" while the file was still searchable.

**`DetailFields` fell back to raw JSON whenever the search term was not in the fields.** The
fallback exists for a hit that tokenization split across a boundary, but the condition only asked
whether any token contained the term — trivially false when the term is simply not in this record's
`detail`, which is every message search. So typing anything that matched a message turned the new
key/string/number colouring off on exactly the rows being read. It now also requires the term to be
in `detail`, which is the half the comment describes and `LogRow` already computes as
`matchedInDetail`.

Two tests, each failing against the previous behaviour: a mid-run `ENOSPC` keeps its path and
answers from the file, and an open that never succeeded drops it. The token fallback has no test —
the package has no UI component harness — but the condition was verified by replicating
`tokenizeDetail` against a real `detail` string.

Files: `src/services/server/LogStore.ts`, `src/tests/log-store.test.ts`,
`ui/src/operator/Logs.tsx`, `docs/ai/flows/a-log-line-from-the-filler-to-the-dashboard.md`.
