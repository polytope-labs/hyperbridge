# Simplex desktop macOS signing keychain

Signed macOS builds failed on the `macos-latest` (macOS 26) arm64 runner. electron-builder's
`CSC_LINK` path imports the `.p12` into its own keychain, then calls
`security set-key-partition-list -k <.p12 password>` against a keychain that has a random password.
macOS 26 rejects that unlock. The same build signed and notarized on the macOS 15 Intel runner.

The publish workflow now builds the signing keychain itself in the `Import Developer ID certificate`
step. It creates a keychain in `$RUNNER_TEMP` with a random password, imports
`SIMPLEX_MACOS_CERTIFICATE_P12`, sets the key partition list with the keychain password, and exports
`CSC_KEYCHAIN`. The build step no longer sets `CSC_LINK` or `CSC_KEY_PASSWORD`. The keychain is
deleted with the notarization key at the end of the job.

`release-signing.cjs` requires `CSC_KEYCHAIN` for signed macOS builds and rejects `CSC_LINK`, so the
failing electron-builder path cannot come back silently. It also sets `mac.identity` to
`APPLE_TEAM_ID`, which pins signing to the Developer ID identity for that team.

The GitHub secrets are unchanged.

## When the desktop workflows run

`publish-simplex-desktop.yml` no longer runs on pull requests. It runs on a pushed
`simplex-desktop-v*` tag, or on a manual dispatch, which is how a signed build is validated on `main`
before tagging.

`test-simplex-desktop.yml` runs only when `sdk/packages/simplex-desktop/**` or the workflow file
changes. Changes to `sdk/packages/sdk`, `sdk/packages/simplex` or the pnpm lockfile no longer trigger
it.
