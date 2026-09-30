# 2026-09-10 — A failed launch file keeps its path unless the open never succeeded

Decided: when the launch file's write stream errors, `LogStore` always drops the stream but drops
`this.file` only if `stream.pending` is still true.

The question is whether the path on disk holds *this launch's* records. Two failures look alike at
the handler and are not:

- The open failed. The `wx` flag refused a collision with another launch's file, or the directory
  could not be written. `pending` is still true, no write of ours reached that path, and reading it
  back would report another launch's seq-1..N run as this launch's history — which is the bug `wx`
  was added to prevent in the first place. Drop the path.
- The open succeeded and a later write failed — a full disk, most likely, which is the failure a
  history "bounded only by the disk" invites. `pending` is false, the file is ours, and every record
  written before the failure is intact and readable. Keep the path: `recent` goes on answering from
  it, and the hours before the disk filled are what an operator opens the Logs page for.

Why `pending` rather than `err.code === "EEXIST"`: the errno test gets open-time `EACCES` and
`EISDIR` wrong in the opposite direction, retaining a path to a file that was never created, so
`stats()` would advertise a file that is not there. `pending` asks the question actually being asked
— did a file descriptor ever exist — and is documented as exactly that.

Rejected: keeping the path unconditionally. That reintroduces the cross-launch read `wx` closes.

Rejected: leaving the footer's two states alone. With the path retained, "memory only" and its
"only the in-memory tail is searchable" tooltip became false in the case that matters most, so the
footer gained a third state rather than lying about the second.
