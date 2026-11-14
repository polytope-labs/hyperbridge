# Reset Configuration in the desktop menus

The tray menu and the application menu have a **Reset Configuration…** item (`reset-configuration`
in `src/tray-menu.ts`). It returns Simplex to a new installation.

1. A native dialog asks for confirmation. Cancel is the default.
2. The app stops the solver and waits for the process to exit. If the solver cannot be reached or
   does not exit, the reset is abandoned and nothing is deleted.
3. Launch at login is switched off, because that setting lives outside the data directory.
4. The app writes `reset-pending` into the user-data directory and relaunches.
5. The relaunched app finds the marker and deletes every entry in the user-data directory before
   anything opens it (`src/profile-reset.ts`). The marker goes last, so a reset that fails part-way
   is tried again on the next launch.

The deletion happens in the next launch, not the one that confirms, because Chromium keeps files in
the user-data directory open while the app runs and Windows will not delete an open file.

The item is disabled while Simplex is locked and while an update is installing. Between the
confirmation and the relaunch nothing can start the solver again.

The reset deletes the only local copy of the filler wallet's key. The dialog says that the funds
need a backup of the key afterwards.
