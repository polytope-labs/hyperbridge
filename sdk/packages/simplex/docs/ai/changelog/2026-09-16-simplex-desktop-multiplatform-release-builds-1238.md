# Simplex desktop multiplatform releases and native signing (#1238, #1239)

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
and enforces measured installed-size budgets of 520 MiB for macOS and Linux and 580 MiB for Windows
without relaxing the target host's sandbox policy. It assembles stable or beta updater metadata,
merges both macOS update ZIPs into one channel document, recomputes every referenced SHA-512, and
rejects missing or unexpected release assets. Native smoke tests drive first-run config creation and
confirm an offline configured boot fails closed before cleanup.

A release tag fails closed unless macOS has a Developer ID Application certificate and App Store
Connect notarization key and Windows has Azure Trusted Signing credentials. macOS uses hardened
runtime for the app and bundled Node with only the JIT and unsigned-executable-memory entitlements;
CI verifies the app, Electron helpers, and Node team identity and exact entitlements, runs Gatekeeper assessment on the app and
signed DMG, and validates stapled notarization tickets on both. Windows signs and timestamps the app, staged `node.exe`, and NSIS
installer, then verifies the Authenticode publisher of every packaged executable. Signing credentials are never used for pull
requests, including fork pull requests. A maintainer can opt a manual workflow run into the identical
signed path from `main` to validate private artifacts on clean machines before creating a public tag.
Tags must point into `origin/main`, and the matching namespace is intended to be protected by a
repository tag ruleset. Credentials live in a protected `simplex-desktop-release` environment limited
to `main` and the release tag namespace; unsigned CI runs in a separate secretless environment.
On macOS, the updater also requires the installed app's signature to match the Apple Team ID recorded
in signed app metadata before it enables automatic updates.

After every native slice passes, CI assembles the assets in a draft and publishes it as its final
step. Published releases are immutable to the workflow. Linux artifacts remain manually installable,
while Linux automatic updates stay disabled until their channel metadata has an independent
signature. This tag namespace remains independent of the npm and Docker release workflows.
