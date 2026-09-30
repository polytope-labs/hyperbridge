# 2026-09-03 — Clarify setup delegation failures

Setup now translates the internal all-chain EIP-7702 shutdown message into network-aware funding,
RPC/bundler, retry, and image-version guidance while retaining raw messages for unrelated startup
failures. The failed-state action is labelled Retry startup and makes clear that the configuration
was already saved.

Files: `ui/src/wizard/startError.ts`, `ui/src/wizard/steps/Review.tsx`,
`src/tests/setup-completion.test.ts`, and `docs/ai/{ChangeLog,Decisions,Flow}.md`.
