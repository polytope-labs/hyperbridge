# Simplex desktop multiplatform release builds (#1238)

Simplex Desktop now has locked, repeatable native packaging for macOS arm64 and x64, Windows x64, and
Linux x64 and arm64. Electron and `electron-builder` are pinned exactly, with release-only tooling in
its own locked workspace so installer dependencies do not enter the SDK graph.

Packaged applications keep the Electron main process in `app.asar` while placing the built Simplex
solver and UI, its production dependency closure, tray resources, and the independently verified
Node 24.19.0 executable as plain files under Electron resources. A packaging hook selects the staged
runtime for the target architecture, preserves executable permissions, and rejects an incomplete
resource layout before an installer is produced.

The `simplex-desktop-v*` release workflow builds each target on a matching native runner, carries
forward recursive submodule checkout and protoc setup, launches the unpacked application and every
native artifact, checks the setup UI and both structured logs and captured solver stderr, and
enforces a 520 MiB installed-size budget. It assembles the platform installers and electron-updater
channel files for stable and beta releases, merges both macOS update ZIPs into one channel document,
recomputes every referenced SHA-512, and publishes the complete set to a version-matched GitHub
release. This tag namespace is independent of the npm and Docker release workflows. Signing and
notarization remain the responsibility of #1239.
