# 2026-11-23 — CI no longer installs the SP1 toolchain

`Check Workspace` and `Unit Tests` no longer run the SP1 installer, `sp1up` or
`cargo prove --version`.

No crate builds an SP1 program. `sp1-beefy` embeds its prebuilt program with
`include_bytes!`, and the SP1 crates' build scripts do not need the toolchain.

Each install unpacked a new ~1.2 GB toolchain into `~/.sp1/toolchains/<random>` and never
removed the old one. The `arc-runner-set` runners keep `$HOME` between jobs, so by
2026-11-23 the four runners held about 1,700 copies, roughly 2 TB.

If CI ever needs to build an SP1 program again, clear the old `~/.sp1/toolchains`
directories before each install, because these runners keep `$HOME`.
