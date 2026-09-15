# Simplex desktop lifecycle and OS integration (#1243)

The Simplex desktop shell now stays available from a native tray after its
window closes. The tray and application menus report solver and sleep-prevention
state, expose pause, resume, stop, and restart controls, open the data directory
and current rotating log, and clearly separate quitting the app from stopping
the solver and quitting. The tray artwork is derived from the existing PWA icon
on macOS, Windows, and Linux.

The shell checks the private socket every three seconds, reports a solver that
has stopped or become unreachable, and offers an operator-controlled restart.
It prevents app suspension only while the solver is actively filling. Optional
login startup registers the installed app—not the solver—through Electron on
macOS and Windows and through per-user XDG autostart on Linux.

No remote crash reporter is enabled: diagnostics remain in the existing five
rotating launch logs. Desktop removal preserves the user-data directory by
contract so configuration, bid records, and reclaimable-deposit history are not
silently deleted. The documentation also makes clear that login startup and
sleep prevention are not substitutes for running the Simplex container on a
VPS when continuous uptime is required.
