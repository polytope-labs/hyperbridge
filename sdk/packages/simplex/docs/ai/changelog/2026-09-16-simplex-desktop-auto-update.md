# Simplex desktop auto-update

Installed Simplex desktop builds use `electron-updater` with a GitHub provider that filters the
`polytope-labs/hyperbridge` release API to `simplex-desktop-v*` tags. This prevents unrelated
monorepo releases from becoming desktop update candidates. Stable is the default channel and beta is
opt-in. Checks and downloads run while the detached solver continues filling, but ordinary Electron
quit never installs an update.

`GET /health` now reports the solver PID and operator `GET /api/status` reports queued and active
evaluation, queued and active fill, retraction, and rebalancing counts. A downloaded update waits for
active work to drain; running solvers must also have no queued evaluations, while paused solvers may
discard not-yet-started evaluations through the existing graceful-stop behavior. Rebalancing promises
are tracked and drained during graceful stop. The installer runs only after the private socket and old
PID are both gone. A slow drain is deferred without killing the solver, and a staged update older than
24 hours raises a daily notification.

The app persists the target version before installation and clears it only when the relaunched app
and healthy setup or operator solver report the same version. The custom protocol exposes the
immutable desktop version as a response header; the renderer blocks setup and operator pages when it
differs from the solver version and directs the operator to restart with the bundled solver. A solver
from before PID health reporting can still be attached and gracefully restarted by treating socket
release as its exit proof, but automatic installation requires a reported PID. Rollback uses a
previous signed installer after a graceful solver stop; channel changes ignore downloads started on
the previous channel and never trigger an automatic downgrade.

After an Electron restart, a saved download receipt triggers another updater check so the current
process revalidates and reopens the cached artifact before the solver is stopped. Installer errors
clear the attempted marker, restart a solver that the updater stopped, and leave the download
retryable. A failed post-install version check is notified once before normal checks resume. Updates
remain staged while the operator has intentionally left the solver stopped, so a background update
cannot silently resume filling. Desktop builds fail unless the desktop and bundled Simplex manifests
have the same version.

Release metadata supplies SHA-512 artifact verification. macOS also verifies the application code
signature, and Windows NSIS updates keep Authenticode publisher verification enabled. Installer and
signing production remain responsibilities of the desktop release build.
