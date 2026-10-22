# 2026-10-22 — Indexer dependencies are pinned to exact versions

Every dependency in the indexer's `package.json` names an exact version, except the
`workspace:*` link to `@hyperbridge/sdk`. None of them use `latest` or a range any more.

The pins are the versions `sdk/pnpm-lock.yaml` already resolved, so installs and the release build
are unchanged. `viem` is pinned to 2.47.6, the version the workspace override in
`pnpm-workspace.yaml` already forces.

The release tarball has its own `package.json`, written by `.github/workflows/publish-indexer.yml`.
Its six dependencies are pinned to the same versions. A deployment installs them with `npm install`
and no lockfile, so a range there would resolve fresh on every deploy.

A `latest` specifier takes whatever npm tags as latest on the day the lockfile is refreshed.
`@subql/common@5.8.3`, published on 2026-10-05, carried a credential-stealing payload and was briefly
tagged `latest`. Upgrade a dependency by changing its version here, deliberately. When the change
touches one of the release package's six dependencies, change it in the workflow too.
