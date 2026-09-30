# 2026-09-10 — The log history is a file; memory is a cache in front of it

Decided: every captured record is appended to `<dataDir>/logs/simplex-<launch>.log` as NDJSON, and
the last 5000 are also kept in a ring. A query is answered from the ring unless it reaches past the
oldest record still there, in which case the older half is read back off the file.

"Everything since launch" and "does not cost the filler its heap" cannot both come from one
in-memory structure. At trace level under load a filler produces records faster than any bounded
ring can retain, and an unbounded one is a slow OOM in a process whose actual job is bidding on
intents. The file has no such ceiling and is greppable by anything, not only the dashboard.

The ring stays because the live stream broadcasts from it, and because the common case — a filler at
`info` whose whole launch still fits — never touches disk at all. The split point is the ring's
oldest seq, which also sidesteps the write stream's buffer: the records not yet flushed to disk are
exactly the ones the ring still holds, so the two halves cannot miss a record between them. The ring
is also checked *first*: when it alone fills the requested page, nothing older could survive the
final slice, and the scan is the expensive part.

Rejected: SQLite, which the package already uses for bids and activity. Log records are append-only,
never updated, and queried by substring — none of which needs a database, and each insert would be
synchronous work on the logging path.

Rejected: keeping only the file and dropping the ring. Every keystroke in the search box would be a
full file scan, and the live stream would have nothing to broadcast from.

Launch files are pruned to the last five, and each carries a millisecond-resolution stamp opened
`wx`. A second-resolution stamp with an appending stream let a `RestartSec=0` restart write a second
seq-1..N run into the dead launch's file, which `scanFile` then read back as this launch's history.

A single launch is not size-capped: capping it would make "all logs from launch" false, and the
level control is the intended lever on volume.
