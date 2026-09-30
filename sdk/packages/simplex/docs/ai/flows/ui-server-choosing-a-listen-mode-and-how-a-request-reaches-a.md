# UI server: choosing a listen mode, and how a request reaches a handler

Read from the source and exercised by `src/tests/ui-server-socket.test.ts`; the Node behaviours noted
below were measured on Node 24 rather than inferred.

1. `bin/simplex.ts` resolves one listen target before anything binds. The flags themselves are
   declared in `src/cli/run-options.ts` (`addRunOptions`), so they can be parsed in tests without
   importing the bin. `--no-ui` skips the server entirely; `--ui [host:port]` fills `uiBind` (default
   `127.0.0.1:8686`); `--ui-socket <path>` selects socket mode. `--ui-socket` with either `--no-ui` or
   an explicit `--ui <addr>` throws before the filler starts, rather than one silently winning. A bare
   `--ui` is not a conflict: it names no address, it only turns the UI on.
2. `UiServer.start()` dispatches on the argument's shape: `start(port, host)` and `start({host, port})`
   both reach `listenOnPort`, `start({socketPath})` reaches `listenOnSocket`. The TCP path is
   unchanged — the init-mode loopback refusal, the operator-mode warning, `boundLoopback`, and the
   resolved bound port all behave as before, and the ephemeral-port retry in the wizard path still
   works by passing port 0.
3. `listenOnSocket` resolves the path (unless it is a `\\.\pipe\` name), checks it against
   `sun_path` (103 bytes on macOS, 107 on Linux) and connect-tests any file already there:
   `ECONNREFUSED` means stale, so it is unlinked; anything answering fails the start. Anything at the
   path that is not a socket is refused outright rather than removed, and the check is `lstat`, so a
   dangling symlink is seen rather than read as absent. It then binds through `listenPrivate`, which
   sets `umask 0177` around the synchronous `listen` so libuv creates the file `0600` — Node would
   otherwise create it `0777 & ~umask`, i.e. 0775 under `umask 002`. `assertSocketIsPrivate` then
   asserts the mode (it does not repair it), and resolves 0, since there is no port to report.
4. `listenProvenance` is set by whichever of the two ran (`tcp` / `unix`), inside the `listen` callback
   rather than before the bind, so a rejected start never leaves a live listener described by the
   other mode's rules. A listener prepended to the
   server's `connection` event stamps that value onto every accepted socket, ahead of http's own
   connection listener, so the tag is in place before any parser exists.
5. Tunnel connections skip steps 1–4 entirely. `TunnelService` dials the relay outbound; a device's
   channel is built by `EmbeddedSshServer`, stamped `tunnel` there, and handed to
   `deliver: (socket) => uiServer?.accept(socket) ?? false`. `accept()` refuses when the server is not
   listening, stamps `tunnel` itself (first stamp wins, so the two agree), and emits `connection`. No
   port is involved, which is why a socket-only server serves remote devices exactly as a TCP one does.
6. `handle` reads `provenanceOf(req.socket)` once, then applies the rules in order: the host-header
   (DNS-rebinding) check for everything except `unix`; the `X-Simplex-UI` CSRF header on every
   mutating method regardless of transport; and the refusal to let a `tunnel` connection change remote
   access. Only the first of the three varies by provenance.
7. `stop()` ends every SSE client, closes the server and unlinks the socket path. `server.close()`
   also unlinks after it drains, so the explicit call is what frees the path immediately and covers a
   close that never completes.
