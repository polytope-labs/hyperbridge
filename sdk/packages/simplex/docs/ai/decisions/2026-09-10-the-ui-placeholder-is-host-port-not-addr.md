# 2026-09-10 — The `--ui` placeholder is `[host:port]`, not `[addr]`

Decided: spell the optional value `[host:port]`.

Why not `[addr]`: the placeholder is what `--help` shows, and the flag takes either a bare port
(`--ui 9000`) or `host:port`. `[host:port]` keeps that shape visible; `[addr]` hides it.

Why not keep the old `[host:]port` nuance, where the *host* half is the optional one: any angle
bracket anywhere in the flags string sets commander's `required`, which is the bug being fixed, and
commander has no notation for "optional value whose host half is also optional". The description
carries that instead.
