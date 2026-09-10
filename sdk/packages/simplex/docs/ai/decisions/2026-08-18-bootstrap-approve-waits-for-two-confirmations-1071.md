# 2026-08-18 — Bootstrap approve waits for two confirmations (#1071)

Chosen: `sendFundedApprove` waits for `confirmations: 2`. On Base Sepolia the first sponsored op right after a one-confirmation approve was rejected `AA33` (Permit2 `TRANSFER_FROM_FAILED`: the bundler's simulation node had not seen the approve yet) twice in a row, and the identical op seconds later was accepted; with two confirmations a fresh EOA went through first try. Alternative: retry the op on `AA33` — more code for a once-per-token-lifetime event that costs one extra block.
