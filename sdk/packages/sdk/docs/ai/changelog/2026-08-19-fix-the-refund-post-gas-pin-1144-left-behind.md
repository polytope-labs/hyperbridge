# 2026-08-19 — Fix the refund-POST gas pin #1144 left behind

#1144 split `CANCEL_MESSAGE_GAS = 800_000n` into `SOURCE_GET_RESPONSE_GAS` and `REFUND_POST_GAS`, both 1M, but left `orderCanceller.test.ts` asserting the POST at 800k — main's own CI has failed the concurrent-sdk step since it merged, and every PR cut from it inherited the red check. The pin now matches the shipped constant, with a comment naming the origin so the next reprice updates both.

Files: `src/tests/orderCanceller.test.ts`.
