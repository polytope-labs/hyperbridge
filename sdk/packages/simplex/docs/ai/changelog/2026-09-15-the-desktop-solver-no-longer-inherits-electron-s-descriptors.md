# 2026-09-15 — The desktop solver no longer inherits Electron's descriptors

On Linux, the solver that `simplex-desktop` spawns used to start with a copy of Electron's open
file descriptors. Electron's main process leaves them without close-on-exec, and libuv forks
without closing them. The solver then held them for its whole life: Electron's stdout and stderr,
Chromium's sockets and shared memory, its resource files, and any DevTools listener.

`spawnDaemon()` now fills every open descriptor slot above stdio with `/dev/null` on Linux. libuv
`dup2()`s each slot in the child, which replaces the inherited copies. `linuxDaemonStdio()` builds
that stdio array from `/proc/self/fd`. macOS and Windows still pass `stdio: "ignore"`. On macOS,
libuv spawns with `POSIX_SPAWN_CLOEXEC_DEFAULT`, so nothing leaks.

Windows is unchanged. libuv always spawns there with handle inheritance, so the solver can still
receive any of Electron's handles that are marked inheritable, such as standard handles redirected
by whatever launched Electron.

This leak is why the Linux Electron E2E jobs hung until the job timeout. The inherited DevTools
listener failed the no-TCP check. Cleanup then called Playwright's `close()`, which waits for
Electron's stdio pipes to close, and the solver still held them.

The E2E suite changed with it:

- The lifecycle test asserts on Linux that the solver holds no descriptor into Electron's install
  directory. Against the old spawn it fails in about a second and lists the leaked files.
- Quitting and killing Electron wait for the process to exit, not for its stdio to close.
- The hard kill targets Electron's own main process ID, read with `process.pid` inside Electron. On
  Windows, Playwright launches Electron through `cmd.exe`, so its process handle is the shell.
  Killing the shell left Electron running with the single-instance lock, which made the relaunch
  quit at once.
- The Windows listener check prints the collected ports as its last statement.
  `Get-NetTCPConnection` reports "nothing found" as an error, so PowerShell exited 1 in the passing
  case.
- Cleanup stops any process still running the Electron binary, logs its `--type`, and retries
  deleting the profile directory.
- `test:e2e` passes `--test-timeout=300000`, so a stuck test is named.

The workflow now checks out `github.sha`, the PR merge commit, instead of the PR head. It runs the
desktop scripts with `pnpm --dir packages/simplex-desktop`. Before, a branch that predated the
package ran `pnpm --filter`, which matched nothing, exited 0, and reported the E2E green.

Files: `sdk/packages/simplex-desktop/src/daemon.ts`, `src/tests/daemon.test.ts`,
`scripts/e2e/desktop.e2e.mjs`, `package.json`, and `.github/workflows/test-simplex-desktop.yml`.
