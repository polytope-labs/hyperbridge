# 2026-09-21 — Every bid has its own nonce key

`SolverAccount.validateUserOp` requires a selected bid's nonce key to be

```
uint192(keccak256(commitment ‖ sessionKey ‖ keccak256(op.callData)))
```

where it was `uint192(keccak256(commitment ‖ sessionKey))`.

A solver bidding several prices on one order signs one UserOp per price. With the key shared by the
order, those ops were sequences of one key, and the EntryPoint runs a key's sequences strictly in
order: a bid that was never selected, or expired, blocked every bid signed after it, so a solver's
bids on an order could not execute independently. Keyed by its calldata, each bid is sequence 0 of
its own key and executes on its own, in any order.

The key is still derived entirely from what the op carries — the commitment in its signature, the
session key the gateway's `select` recovers, and its calldata — so nothing in it is the solver's to
choose. The same calldata still executes once: the EntryPoint consumes the key's first sequence.

Hashing the calldata costs about 1.1–1.7k gas per validation on a realistic bid (1.9–3.5 KB of
calldata), where decoding the `fillOrder` inside it would cost 4.4k–15k.

`CryptoUtils.bidNonceKey(commitment, sessionKey, callData)` is the SDK's side of the derivation, and
`SolverAccountTest.test_BidNonceKey_MatchesSdkVector` pins the two to the same vector.
