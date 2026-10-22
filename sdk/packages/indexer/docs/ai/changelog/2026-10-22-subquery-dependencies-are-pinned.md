# 2026-10-22 — SubQuery dependencies are pinned to exact versions

Every `@subql/*` dependency in `package.json` names an exact version. None of them use `latest` any
more.

The pins are the versions `sdk/pnpm-lock.yaml` already resolved, so installs and the release build
are unchanged.

A `latest` specifier takes whatever npm tags as latest on the day the lockfile is refreshed.
`@subql/common@5.8.3`, published on 2026-10-05, carried a credential-stealing payload and was briefly
tagged `latest`. Upgrade a SubQuery package by changing its version here, deliberately.
