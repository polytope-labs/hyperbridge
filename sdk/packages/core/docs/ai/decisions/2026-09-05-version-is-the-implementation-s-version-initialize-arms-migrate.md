# 2026-09-05 — `version()` is the implementation's `VERSION`; `initialize` arms, `migrate` catches up

Chosen: one constant, `VERSION = 2`, that both `initialize` and `migrate` land on. `initialize`
takes the relayer in the init data, so a fresh proxy is armed from its first block and reports
the version of the code it runs. `migrate`, host-only and one-shot, exists for proxies deployed
before this implementation and reverts on any proxy already at `VERSION`. `setRelayer` is a
rotation and never touches the version. `onlyHost` on `migrate` is load-bearing: a proxy at 1 is
open, and without it anyone could arm it first.

This reverses the earlier decision (below, same day) to keep the relayer out of the init data.
The reason given there, that the relayer would become part of what fixes the proxy's CREATE2
address, still holds but no longer bites: the implementation address is already an input to
that address, so every new implementation changes it for chains deployed afterwards anyway, and
the deploy script now deploys a proxy only where none exists. Landing fresh proxies at 1 with an
open gate, the state before this change, left them reporting an older version than their code.

Alternative rejected — `setRelayer` under `reinitializer(_getInitializedVersion() + 1)`, built
and tested first. It makes `version()` count key rotations, which says nothing about what code a
proxy has migrated to.

Alternative rejected — a fixed `reinitializer(2)` on `setRelayer` itself: the second rotation
reverts until an implementation with `reinitializer(3)` ships.

Alternative rejected — a separate `bumpVersion()` next to a plain `setRelayer`: an
`UpgradeContract` carries one migration call, so arming a fresh chain would take two deliveries.

The bytes came from deduplicating internal code, not from dropping anything off-chain reads. The
one place `_sendValue` is not used is the `_fillSameChain` loop, which is at the via-ir stack
limit.
