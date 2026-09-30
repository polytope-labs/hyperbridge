# 2026-08-18 — `viemSigner` derives `signAuthorization`, and rejects only an account that can neither authorize nor sign digests

Chosen: viem makes `signAuthorization` optional on an account; the interface does not. The adapter uses the account's own when present (private keys, Turnkey), falls back to hashing the tuple and signing it with `sign`, and throws at construction when the account has neither.

Alternative considered: failing lazily at the first delegation attempt.

Why: solver selection is the only fill path simplex uses, and it requires a signed EIP-7702 authorization. An account that cannot produce one yields a solver that boots, scans, and fails at its first delegation — a failure separated from its cause by everything in between. Watch-only solvers do not build a signer at all (boot stands a throwaway key in), so nothing legitimate is blocked by failing early.
