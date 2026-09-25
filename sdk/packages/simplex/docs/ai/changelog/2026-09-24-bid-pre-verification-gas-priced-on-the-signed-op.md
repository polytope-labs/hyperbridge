# 2026-09-24 — A bid's preVerificationGas is priced on the op it signs

Bids on Base were admitted by Alchemy's bundler (`eth_sendUserOperation` returned a hash) and
then never bundled. On an L2, `preVerificationGas` also pays the L1 data fee for the op's bytes:
a fixed amount of wei the bundler converts to gas at the price the op pays. The bundler admits
an op priced at its `maxFeePerGas`, but bundles it at the base fee plus its priority fee, where
the same wei costs more gas, and skips an op short of that until it expires.

Bids signed the `preVerificationGas` of the shared fill estimate, which priced a lone
`fillOrder` at the op's max fee. The signed bid also carries its funding calls and the approval
pair (about a third of its calldata on mainnet) and is bundled below its max fee.

`ContractInteractionService.prepareBidUserOp` now asks the bundler for the bid's
`preVerificationGas` once the calldata and `paymasterAndData` are final, through the SDK's
`IntentGateway.estimateBidPreVerificationGas` (`GasEstimator.estimateBidPreVerificationGas`):

- The op sent is the bid's own: its calldata, gas limits, nonce and `paymasterAndData`, with a
  162-byte signature (commitment, solver signature, session selection) and `preVerificationGas`
  zero, since bundlers return a non-zero value as given.
- It is priced at the bundle's price: the latest base fee plus the op's priority fee, capped at
  its `maxFeePerGas`.
- A state override replaces the solver account's code with `BID_PVG_ESTIMATION_ACCOUNT_CODE`,
  which answers `isValidSignature` with the ERC-1271 magic value (Permit2 checks the paymaster's
  permit through it) and every other call with 32 zero bytes. Validation passes and `execute`
  does nothing, so only the op's bytes are priced; the fill itself cannot pass in simulation
  without the session's selection, and funding calls do not resolve there.
- `BID_PVG_HEADROOM_PERCENT` (10%) is added for L1 prices moving before the bid executes.

If the estimate fails, the bid keeps the fill estimate's `preVerificationGas` and logs a warning.
The paymaster's permit amount does not depend on `preVerificationGas`, so `paymasterAndData` is
built once, before the estimate.
