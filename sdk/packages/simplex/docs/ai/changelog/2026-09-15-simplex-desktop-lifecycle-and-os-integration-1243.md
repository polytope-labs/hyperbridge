# Simplex desktop lifecycle and OS integration (#1243)

The Simplex desktop shell now stays available from a native tray after its
window closes. The tray and application menus report solver and sleep-prevention
state, expose pause, resume, stop, and restart controls, open the data directory
and current rotating log, and clearly separate quitting the app from stopping
the solver and quitting. The tray artwork is derived from the existing PWA icon
on macOS, Windows, and Linux.

The shell checks the private socket every three seconds, reports a solver that
has stopped or become unreachable, and offers an operator-controlled restart.
The socket remains bound and reports `stopping` until graceful shutdown has
drained in-flight fills and vault redemptions; restart stays disabled throughout
that interval. Transient health failures require two consecutive probes, while
a stale Unix socket after an unexpected exit raises a crash alert. It prevents
app suspension only while the solver is actively filling. Optional login
startup registers the installed app—not the solver—through Electron on macOS
and Windows and through per-user XDG autostart on Linux AppImage's stable path.

No remote crash reporter is enabled: diagnostics remain in the existing five
rotating launch logs. Desktop removal preserves the user-data directory by
contract so configuration, bid records, and reclaimable-deposit history are not
silently deleted. The documentation also makes clear that login startup and
sleep prevention are not substitutes for running the Simplex container on a
VPS when continuous uptime is required. Setup mode can also be stopped when
save-and-start is not in progress. The native application menu preserves Edit
and Window roles, unsigned Windows builds avoid a path-sensitive tray GUID, and
macOS ships 18px plus 36px Retina template icons.
