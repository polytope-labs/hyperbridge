# 2026-09-03 — `HyperFungibleToken` opts in to the relayer gate; `BridgeToken` fails closed

Chosen: the base token's `_checkRelayer` only rejects when a relayer has been set, and
`BridgeToken` overrides it so that an unset relayer matches nobody.

`HyperFungibleToken` is a library contract that third parties deploy from this package. Failing
closed there would leave every token deployed without a `setRelayer` call unable to receive
anything, with no compile-time signal. The BRIDGE token is ours, its supply is backed by the nexus
escrow, and a forged mint is exactly the attack the gate exists for, so it takes the strict
semantics of the intent gateway. The two behaviours live in one virtual function so the difference
is visible in one place rather than spread through the callbacks.

Alternative rejected — override `onAccept` in `BridgeToken`. It is `external`, so an override
cannot call the parent body and would have to duplicate the mint logic.

Alternative rejected — make the base fail closed and bump the package major. Correct in principle,
but the request was for the bridge token, and the base can be tightened later once every
deployment from this package has a relayer set.
