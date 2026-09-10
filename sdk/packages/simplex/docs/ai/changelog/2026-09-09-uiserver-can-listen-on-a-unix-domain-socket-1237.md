# 2026-09-09 — UiServer can listen on a Unix domain socket (#1237)

`UiServer.start()` now takes a listen target — `{ host, port }` as before, or `{ socketPath }` — and
`simplex --ui-socket <path>` selects the second from the CLI. Node's `listen(path)` speaks HTTP over a
Unix socket natively, so the route table, the handlers and the SSE stream are untouched; the daemon can
be embedded by a desktop app with no TCP port open at all. The CLI's default is unchanged: `--ui` and
`--no-ui` behave exactly as before, and `--ui-socket` conflicts with either an explicit `--ui <addr>`
or `--no-ui` rather than silently winning. Verified at the parser level against the real commander
build: for every pre-existing argv, the parse result is identical with and without the new option.

That check also turned up a pre-existing bug: `--ui`'s declared flags, `[<[host:]port>]`, contain a
`<`, and commander computes `required` as `flags.includes("<")`, so a valueless `--ui` was rejected
with "argument missing" and the optional-argument form had never worked. It was left out of this
change to keep `--ui` untouched, and fixed separately in #1249, which also moved the `run` flags into
`src/cli/run-options.ts`. `--ui-socket` is declared there with the rest of them.

The socket is created `0600`. This is the point of the mode, and Node does not do it: `listen(path)`
creates the file `0777 & ~umask`, which is 0775 under the common `umask 002` — measured, not assumed —
so otherwise every member of the operator's group could connect and drive `/api/send`, an
unauthenticated token transfer. Linux and macOS both check write permission on the socket file at
connect(2), so the mode is enforced. (The first version of this change chmod'ed after binding; see the
security-audit entry below for why that was not sufficient.)

Stale sockets are recovered rather than fatal. A `SIGKILL`ed run leaves the file behind and the next
bind fails `EADDRINUSE` (both verified against a real killed process), so the path is connect-tested
first: `ECONNREFUSED` means the file outlived its listener and is unlinked; anything that answers is a
running instance and is refused, which doubles as the single-instance lock. The path is also unlinked
on clean shutdown. Over-long paths are rejected up front with the limit and a suggested fallback —
without the guard they fail at bind with `listen EINVAL: invalid argument`, naming neither cause nor
limit.

`http-util`'s `VIA_TUNNEL` marker is generalised into a `PROVENANCE` symbol carrying `tcp` / `unix` /
`tunnel`, stamped on sockets this process owns and therefore unforgeable in the same way. The
host-header (DNS-rebinding) check is now skipped for socket-arrived connections — there is no name to
rebind onto a socket and no browser that can open one — and is byte-for-byte unchanged for TCP.
`isTunnelled()` keeps its signature and meaning.

Also reworded `EmbeddedSshServer`'s "No UI to serve behind the tunnel" warning, which fires whenever
the server declines a channel and therefore misreports a UI that exists but is not listening.

New `src/tests/ui-server-socket.test.ts` (8 tests) covers the API, mutating routes, CSRF and SSE over a
socket; the host-header skip alongside proof that TCP still rejects the same Hosts; the 0600 mode and
unlink-on-stop; stale recovery and live refusal; the path-length error; and — the trap the issue calls
out — that a socket-only server still serves tunnelled connections, which arrive injected rather than
listened for. Each fix was mutation-checked: reverting it fails its test.

Files: src/services/server/UiServer.ts, src/services/server/http-util.ts,
src/services/tunnel/EmbeddedSshServer.ts, src/bin/simplex.ts, src/tests/ui-server-socket.test.ts,
docs/ai/{ChangeLog,Decisions,Flow}.md.
