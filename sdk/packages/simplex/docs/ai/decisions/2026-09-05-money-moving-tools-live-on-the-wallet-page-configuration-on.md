# 2026-09-05 — Money-moving tools live on the Wallet page, configuration on Operations

Chosen: Send funds and Vault treasury moved from Operations to the Wallet page, above the ledger
they write to, as `WalletTools`; Operations keeps the allowlist and chain editors. Requested by the
maintainer: the tools that move funds belong with the wallet's balances and history.

Alternatives rejected: leaving them on Operations kept "funds and configuration" as one grab-bag
page; duplicating the links on both pages would have two entry points to the same sheet state.
Cross-page navigation (the vault editor's Enable chain link) goes through `Operator` state rather
than a global router, since the dashboard has no URL routing today.
