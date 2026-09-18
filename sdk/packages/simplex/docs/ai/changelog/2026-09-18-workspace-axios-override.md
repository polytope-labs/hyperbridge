# 2026-09-18 — Workspace axios override

`sdk/pnpm-workspace.yaml` now overrides every axios in the workspace graph:

```yaml
'axios@>=1': ^1.18.0
'axios@<1': '>=0.33.0 <1'
```

No workspace package depends on axios directly. It arrives through tronweb, `@binance/common`, the
subql packages, stellar-sdk, soroban-client and hardhat-deploy.

The floors are the first releases that fix the axios advisories published on 2026-07-20, such as
CVE-2026-67320, where the Node HTTP adapter can use an inherited proxy. They also include the
earlier fixes for CVE-2026-44486, 44487, 44488 and 44494.

Each key matches one major version. A consumer on axios 1.x stays on 1.x, and a consumer on 0.x
stays on 0.x. The 0.x consumers (subql, stellar-sdk, hardhat-deploy) move to the 0.x security
line, which avoids the 1.0 API changes.

The ranges go through `minimumReleaseAge`, so the lockfile gets the newest release that is at least
30 days old. On 2026-09-18 that is axios 1.19.0 and 0.33.0.

The simplex CLI bundle inlines tronweb and `@binance/common` together with their axios. That bundle
ships in the npm package and in the desktop app, so this override decides which axios they ship.
The bundle now contains one copy, axios 1.19.0. Before this change it had two, 1.13.5 and 1.15.0.

The override applies only inside this workspace. A project that installs `@hyperbridge/sdk` from
npm resolves tronweb on its own. A fresh install of `tronweb@^6.2.0` gets 6.4.0 or later, which
pins axios 1.18.0.

pnpm 11 reads `overrides` only from `pnpm-workspace.yaml`. It ignores a `pnpm` field in
`package.json`.

Files: `sdk/pnpm-workspace.yaml`, `sdk/pnpm-lock.yaml`,
`sdk/packages/simplex-desktop/scripts/workspace-policy.test.ts`.
