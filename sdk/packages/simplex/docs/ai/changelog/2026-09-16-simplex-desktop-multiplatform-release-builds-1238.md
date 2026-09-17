# Simplex desktop multiplatform release builds (#1238)

Simplex Desktop now has locked, repeatable native packaging for macOS arm64 and x64, Windows x64, and
Linux x64 and arm64. Electron and `electron-builder` are pinned exactly, with release-only tooling in
its own locked workspace so installer dependencies do not enter the SDK graph.

Packaged applications keep the Electron main process in `app.asar` while placing the built Simplex
solver and UI, its production dependency closure, tray resources, and the independently verified
Node 24.19.0 executable as plain files under Electron resources. A packaging hook selects the staged
runtime for the target architecture, preserves executable permissions, and rejects an incomplete
resource layout before an installer is produced.

The release workflow builds portable JavaScript once on Linux, then packages and launches every
target on a matching native runner. The complete matrix runs on packaging pull requests as well as
`simplex-desktop-v*` tags, checks the setup UI and both structured logs and captured solver stderr,
and enforces a 520 MiB installed-size budget without relaxing the target host's sandbox policy. It
assembles stable or beta updater metadata, merges both macOS update ZIPs into one channel document,
and recomputes every referenced SHA-512.

A tag attaches the verified asset set to a draft GitHub release. Unsigned artifacts are not exposed
to the automatic updater: public releases wait for #1239's macOS and Windows signing and for
independently signed Linux update metadata. This tag namespace remains independent of the npm and
Docker release workflows.
