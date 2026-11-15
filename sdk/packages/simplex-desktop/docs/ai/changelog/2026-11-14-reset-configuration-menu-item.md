# Reset Configuration in the desktop menus

The tray menu and the application menu have a **Reset Configuration…** item (`reset-configuration`
in `src/tray-menu.ts`). It returns Simplex to its first launch.

1. The user-data directory is checked first, so a directory the reset refuses is refused before
   anything is paused or stopped. Then a native dialog asks for confirmation. Cancel is the default. The dialog names the directory
   that will be deleted. It also says how many bids still hold a deposit on Hyperbridge, read from
   the solver's `GET /api/activity/bids`, or that it could not check.
2. The app counts the deposits again, because the solver kept bidding while the dialog was open
   (`settleBidsHoldingDeposits` in `src/solver-supervisor.ts`). It pauses the solver, waits up to 20
   seconds for what is in flight, and reads the count, which can then not rise before the solver
   stops. If more bids hold deposits than the dialog said, or the count can no longer be read, a
   second dialog says so and asks again. Cancel there resumes a solver this step paused. So does a
   reset abandoned in the next step while the solver is still up.
3. The app stops the solver and waits for the process to exit. If the solver cannot be reached,
   reports no process id or does not exit, the reset is abandoned and nothing is deleted.
4. The app writes `reset-pending` into the user-data directory. From here the reset is certain.
5. Launch at login is switched off, because that setting lives outside the data directory, and the
   app relaunches.
6. The relaunched app finds the marker and deletes every entry in the user-data directory before
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

The reset deletes the only local copy of the solver wallet's key, and the bid records the solver
retracts from to reclaim deposits. The dialog says both.

A reset is refused when the user-data directory is the home directory or a directory above it,
which `--user-data-dir` allows. A `reset-pending` file found in such a directory is ignored. The two
directories are compared by their true paths, so a link to the home directory and, on macOS and
Windows, the home directory spelled in another case are refused too. A user-data directory that is
itself a link is refused whatever it points at.

Known limits:

- A second copy of Simplex launched during the relaunch also runs the deletion, because the
  deletion comes before the single-instance lock. Taking the lock first would put Chromium's lock
  files into the directory being deleted.
- A copy of the user-data directory made while `reset-pending` is in it carries the marker. Put
  back, it is deleted on the next launch.
- Quitting Simplex while it waits for the solver to stop abandons the reset. Nothing is deleted.
- A solver paused for the count stays paused when it can no longer be reached to resume it: when
  Simplex is quit while the second dialog is open, or when the solver stops but the reset is then
  abandoned. The solver keeps a pause across a restart, and **Resume** in the menu ends it.
