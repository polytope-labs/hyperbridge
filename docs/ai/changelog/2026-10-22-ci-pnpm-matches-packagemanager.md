# 2026-10-22 — CI installs the pnpm version the workspace names

Every workflow that sets up pnpm 11 now asks `pnpm/action-setup` for `11.0.0`, the version in
`sdk/package.json`'s `packageManager` field. Before this, the workflows asked for `11`, which
means the newest 11.x release.

When the running pnpm differs from `packageManager`, pnpm downloads `@pnpm/exe` at the named
version and switches to it. Since pnpm 11.28.7 (2026-10-21), that download goes through the
workspace's `trustPolicy: no-downgrade`. `@pnpm/exe@11.0.0` has a provenance attestation where
the releases before it had a trusted publisher, so the check refuses it and `pnpm install` fails
before resolving anything.

Running the exact version means pnpm never switches. Change the workflows and `packageManager`
together when upgrading pnpm.
