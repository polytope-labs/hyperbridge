# Simplex Desktop solver start timeout

Simplex Desktop now waits up to five minutes for the solver's `/health` to report
ready, raised from 120 seconds. The deadline is `SOLVER_READY_TIMEOUT_MS` in
`sdk/packages/simplex-desktop/src/daemon.ts` and applies wherever the desktop
starts or attaches to the solver: at launch with the OS-protected restart key,
during unlock, from the tray restart actions, and when restarting after an update.

The timeout error now reports 300 seconds. On timeout the detached solver is left
running, and the unlock still completes only after the solver is ready.
