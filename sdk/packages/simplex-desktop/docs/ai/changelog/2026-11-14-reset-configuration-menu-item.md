# Reset Configuration in the desktop menus

The tray menu and the application menu have a **Reset Configuration…** item (`reset-configuration`
in `src/tray-menu.ts`). It returns Simplex to its first launch.

1. A native dialog asks for confirmation. Cancel is the default. The dialog names the directory
   that will be deleted. It also says how many bids still hold a deposit on Hyperbridge, read from
   the solver's `GET /api/activity/bids`, or that it could not check.
2. The app stops the solver and waits for the process to exit. If the solver cannot be reached,
   reports no process id or does not exit, the reset is abandoned and nothing is deleted.
3. The app writes `reset-pending` into the user-data directory. From here the reset is certain.
4. Launch at login is switched off, because that setting lives outside the data directory, and the
   app relaunches.
5. The relaunched app finds the marker and deletes every entry in the user-data directory before
   anything opens it (`src/profile-reset.ts`). Every entry is tried, and an entry that another
   process still holds is retried for up to three seconds. The marker goes last and only when
   nothing else is left, so a reset that fails part-way is tried again on the next launch, and
   until then the app refuses to start.

The deletion happens in the next launch, not the one that confirms, because Chromium keeps files in
the user-data directory open while the app runs and Windows will not delete an open file.

`resetAvailable` in `src/tray-menu.ts` decides when a reset may start: the session is unlocked, no
reset is under way, and no update is stopping the solver or installing. The menu item uses it, and
so does the action, before the dialog and again after it. While a reset is under way the item reads
"Resetting Configuration…", nothing can start the solver, and a staged update does not install.

The reset deletes the only local copy of the filler wallet's key, and the bid records the solver
retracts from to reclaim deposits. The dialog says both.

A reset is refused when the user-data directory is the home directory or a directory above it,
which `--user-data-dir` allows. A `reset-pending` file found in such a directory is ignored.

Known limits:

- A second copy of Simplex launched during the relaunch also runs the deletion, because the
  deletion comes before the single-instance lock. Taking the lock first would put Chromium's lock
  files into the directory being deleted.
- A copy of the user-data directory made while `reset-pending` is in it carries the marker. Put
  back, it is deleted on the next launch.
- Quitting Simplex while it waits for the solver to stop abandons the reset. Nothing is deleted.
