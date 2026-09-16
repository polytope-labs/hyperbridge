# Simplex desktop auto-update

Installed Simplex desktop builds use `electron-updater` with the explicitly configured
`polytope-labs/hyperbridge` GitHub release feed and `simplex-desktop-v*` tags. Stable is the default
channel and beta is opt-in. Checks and downloads run while the detached solver continues filling, but
ordinary Electron quit never installs an update.

`GET /health` now reports the solver PID and operator `GET /api/status` reports queued and active
evaluation, queued and active fill, and retraction counts. A downloaded update waits for active work
to drain; running solvers must also have no queued evaluations, while paused solvers may discard
not-yet-started evaluations through the existing graceful-stop behavior. The installer runs only after
the private socket and old PID are both gone. A slow drain is deferred without killing the solver,
and a staged update older than 24 hours raises a daily notification.

The app persists the target version before installation and clears it only when the relaunched app
and healthy solver report the same version. The custom protocol exposes the immutable desktop version
as a response header; the renderer blocks operator pages when it differs from the solver version and
directs the operator to restart with the bundled solver. Rollback uses a previous signed installer
after a graceful solver stop; channel changes never trigger an automatic downgrade.

Release metadata supplies SHA-512 artifact verification. macOS also verifies the application code
signature, and Windows NSIS updates keep Authenticode publisher verification enabled. Installer and
signing production remain responsibilities of the desktop release build.
