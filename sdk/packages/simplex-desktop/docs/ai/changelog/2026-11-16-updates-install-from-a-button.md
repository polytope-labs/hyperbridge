# Desktop updates install from a button

Simplex Desktop still checks for updates and downloads them by itself. It no longer installs them by
itself. A downloaded update waits until the operator asks for it. This replaces the install that ran
once the solver had no work, together with its retry timer and its daily reminder notification.

## Behaviour

- When a download finishes, the updater's state becomes `ready`, with the `targetVersion` it would
  install. The download's SHA-512 has been checked by then. On macOS the system checks the code
  signature later, when the installer runs.
- The dashboard shows an **Update to \<version\>** button at the bottom of the sidebar, above the
  version line. The collapsed sidebar shows the version under the icon. The update item in the tray
  and application menus (`check-for-updates`) becomes **Install Update \<version\>**. Both start
  the same install.
- The install asks the solver to stop, which drains in-flight fills as **Stop solver** does. It
  waits for the private socket and the process to go, with no deadline, because a stop cannot be
  taken back. Then it runs the installer, which relaunches the app.
- The dashboard shows an updating screen in place of its pages while the install runs. The solver it
  reads from is going away, and on macOS the installer fetches the update only after it is invoked,
  so the app stays open for a while with no solver. An install started from the menu shows there
  within one poll.
- While an install runs, nothing can start a solver: the restart items are disabled and
  `startOrAttachSolver` refuses. The reset and the channel selector are unavailable too.
- The app does not check the feed again while a downloaded update waits. A newer release is found
  after the waiting one is installed, or at the next app start.

## When the install does not start

The update stays `ready`. The dashboard shows the reason as a toast, and the menu shows it as a
dialog.

- The solver is not in `setup`, `running` or `paused`. A stopped solver must be restarted first, and
  the menu item is disabled while there is no solver to stop.
- The solver reports no process id, so its exit could not be confirmed.
- A reset is in progress.
- The stop request fails. If the solver stops after all, that is reported like any other solver
  failure, and it is restarted from the menu.

## When the installer fails

This is after the solver was stopped. The app clears the install record and restarts the solver. The
state becomes `error` with the reason, and the download is not offered again by itself. The next
check offers it again: the six-hourly one, **Update Error — Retry** in the menu, or the next start.

A failure that comes at once is the answer to the install request, so it shows as the same toast or
dialog. A failure that comes later, which is the usual case on macOS, shows as a native notification
and as a toast in an open dashboard.

`installing` has no deadline. If the installer neither quits the app nor reports a failure, quit
Simplex and open it again.

## Interfaces

- `GET /api/desktop/update` returns the updater's `UpdateStatus`: `state`, `channel`, and, when they
  apply, `targetVersion`, `progress` and `detail`. The states are `disabled`, `idle`, `checking`,
  `downloading`, `ready`, `stopping-solver`, `installing` and `error`.
- `POST /api/desktop/update` installs the downloaded update. It answers `200` once the installer has
  taken the update. It answers `409` with an `error` when the install did not start, or when the
  installer failed at once. The request stays open while the solver drains.
- Both need an unlocked session and answer `423` without one. The `POST` also needs the
  `X-Simplex-UI: 1` header and the app's own origin, and answers `403` without them.
- The dashboard polls the route every 15 seconds, and every 2 seconds during an install. A browser
  session has no desktop shell and never calls it.
- The `receipt` in `desktop-updates.json` is now written only when an install starts:
  `fromVersion`, `targetVersion` and `installAttemptedAt`. The next launch checks that the app and its
  solver are on `targetVersion`, then clears it. A record left by an older version without
  `installAttemptedAt` is ignored.
