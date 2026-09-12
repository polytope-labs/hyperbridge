# 2026-09-09 — The legacy-database fixtures are committed binaries, not generated in the test (#1236)

Decided: `src/tests/data/fixtures/legacy-v0/{bids.db,activity.db}` are real SQLite files, written
by better-sqlite3 at the schema that shipped before the in-place column migrations, checked into
git. `scripts/make-legacy-db-fixture.mjs` regenerates them and carries the original DDL, copied
verbatim from the commits that introduced it (9149fc52, b3af77e3).

Why committed rather than built in `beforeEach`: the risk being tested is an operator's existing
data directory failing to open under a different driver. A database the test creates through
`node:sqlite` would have been written by the same library that reads it back, so it could not
detect a cross-driver problem even in principle. The generator is not wired into any package
script, because better-sqlite3 is no longer installed — regenerating is a deliberate act that
needs it added back temporarily, which is the right friction for a file whose value is being
*old*.

The root `.gitignore` has a repo-wide `**/*.db`, so the fixture directory carries a `.gitignore`
with `!*.db` to re-include them. That works because no parent directory is excluded, only the
files.

Rejected: a SQL dump replayed at test time. It keeps git free of binaries, but a dump is
re-executed by the current driver, which puts it back in the same category as generating the
database in the test.

Rejected: skipping the fixture and trusting that SQLite's file format is driver-independent. It is
— but the migrations, the WAL header on `activity.db`, and the row values are the parts that
actually break, and none of them are guaranteed by the format.
