# 2026-09-05 — `init` is one-shot, and the EVM deploy scripts do not call it

Chosen: `init` binds the host only while it is unset; a manager constructed with its host set is
bound already.

With the admin surviving `init`, a repeatable `init` would let the governance relayer key re-point
the host and cut it off from its own governance. One-shot closes that. `DeployIsmp.s.sol`
therefore constructs the host first, which works because `EvmHost` takes its params in
`initialize` rather than its constructor, and passes the host to the manager's constructor, so the
relayer key never has to sign a deploy transaction. `TronHost` takes its params in the constructor,
so the Tron migration keeps the `init` route with the deployer as admin until governance rotates
it.

Alternative rejected — precompute the host's CREATE2 address in the script. Works, but couples the
script to the CREATE2 deployer and the host's creation code for no gain over reordering.
