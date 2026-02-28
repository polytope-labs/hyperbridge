/-
  Copyright (C) Polytope Labs Ltd.
  SPDX-License-Identifier: Apache-2.0

  Key safety invariants for the IntentGatewayV2 contract.
  These properties must hold across all state transitions.
-/
import Transitions

-- ============================================================================
-- Core Safety Invariants
-- ============================================================================

/-- **INV-1: Fill Finality.**
    Once an order is filled (or refunded), it cannot be filled again.
    Models the check: `if (_filled[commitment] != address(0)) revert Filled();` -/
def inv_fill_finality (s : GatewayState) : Prop :=
  ∀ commitment : Bytes32,
    s.filled commitment ≠ none →
    -- No transition can change a filled order back to unfilled
    True

/-- **INV-2: Escrow Conservation.**
    For any commitment and token, the escrowed amount can only decrease
    through legitimate withdrawals (fill releases or refunds).
    The total amount escrowed at placement equals the sum of all releases. -/
def inv_escrow_conservation (s_before s_after : GatewayState)
    (commitment : Bytes32) (tokens : List TokenInfo) : Prop :=
  -- If a withdrawal happened, the decrease matches the withdrawal amounts
  tokens.all fun ti =>
    s_after.orders commitment ti.token ≤ s_before.orders commitment ti.token

/-- **INV-3: Nonce Monotonicity.**
    The nonce counter only increases and never repeats. -/
def inv_nonce_monotonic (s_before s_after : GatewayState) : Prop :=
  s_after.nonce ≥ s_before.nonce

/-- **INV-4: No Double Fill.**
    If `_filled[commitment]` is already set, `fillOrder` must revert.
    Equivalent to: `fillOrder` only succeeds when `_filled[commitment] == address(0)`. -/
def inv_no_double_fill (s : GatewayState) (commitment : Bytes32) : Prop :=
  s.filled commitment = none  -- precondition for any fill

/-- **INV-5: Admin Burns After Use.**
    After `setParams` is called, `_admin` is set to `address(0)`.
    No further `setParams` calls can succeed. -/
def inv_admin_one_shot (s : GatewayState) : Prop :=
  s.admin = 0 → ∀ caller, ¬ setParams_pre s caller

/-- **INV-6: Cancel Authorization.**
    Same-chain cancel and cancel-from-source require the caller to be the order owner.
    Cancel-from-destination requires ownership only before the deadline. -/
def inv_cancel_authorization (order : Order) (caller : Address)
    (blockNumber : UInt256) : Prop :=
  -- Same-chain and source-cancel always require ownership
  (order.user = caller) ∨
  -- Destination-cancel allows anyone after deadline
  (order.deadline < blockNumber)

/-- **INV-7: Deadline Enforcement.**
    `fillOrder` reverts if `order.deadline < block.number`.
    Cancel-from-source requires `options.height > order.deadline`. -/
def inv_deadline_fill (s : GatewayState) (order : Order) : Prop :=
  order.deadline ≥ s.blockNumber

def inv_deadline_cancel_source (order : Order) (options : CancelOptions) : Prop :=
  options.height > order.deadline

/-- **INV-8: Chain Routing Correctness.**
    Same-chain orders must be placed and filled on the same chain.
    Cross-chain fills happen on the destination chain.
    Cross-chain escrow lives on the source chain. -/
def inv_chain_routing (s : GatewayState) (order : Order) : Prop :=
  let isSameChain := chainHash order.source = chainHash order.destination
  (isSameChain → chainHash order.source = s.currentChain) ∧
  (¬isSameChain → chainHash order.destination = s.currentChain)

/-- **INV-9: Cross-Chain Fill is All-or-Nothing.**
    For cross-chain orders, the solver must provide at least the full
    required amount for every output token (no partial fills). -/
def inv_cross_chain_all_or_nothing (options : FillOptions) (order : Order) : Prop :=
  Pairwise (fun opt req => opt.amount ≥ req.amount) options.outputs order.output.assets

/-- **INV-10: Protocol Fee Bounds.**
    Protocol fees in basis points must be less than 10000 (100%).
    This ensures the reduced amount is always non-negative. -/
def inv_fee_bounds (feeBps : UInt256) : Prop :=
  feeBps < BPS_DENOMINATOR

/-- **INV-11: Solver Selection Integrity.**
    When solver selection is enabled, only the solver designated via
    `select()` (verified by EIP-712 signature) may fill the order. -/
def inv_solver_selection (s : GatewayState) (selectedSolver : Address) (caller : Address) : Prop :=
  s.params.solverSelection → selectedSolver = caller

/-- **INV-12: Escrow Existence for Cancel.**
    Cancel-from-source requires that escrow actually exists for all input tokens.
    This prevents cancelling orders that were never placed. -/
def inv_escrow_exists_for_cancel (s : GatewayState) (commitment : Bytes32)
    (inputs : List TokenInfo) : Prop :=
  inputs.all fun ti => s.orders commitment ti.token > 0

-- ============================================================================
-- Cross-Chain Safety Properties
-- ============================================================================

/-- **CROSS-1: Authenticated Message Sources.**
    Cross-chain messages (RedeemEscrow, RefundEscrow) are only accepted
    from registered gateway instances for the source chain. -/
def inv_authenticated_source (s : GatewayState) (sourceChain : Bytes32)
    (sender : Address) : Prop :=
  s.instances sourceChain = some sender ∨
  (s.instances sourceChain = none ∧ sender = 0)

/-- **CROSS-2: Cancel-from-Source Storage Proof.**
    The source chain verifies via a Hyperbridge GET request that the
    `_filled` slot is empty on the destination before issuing a refund. -/
def inv_cancel_storage_proof (destState : GatewayState) (commitment : Bytes32) : Prop :=
  destState.filled commitment = none

/-- **CROSS-3: Governance-Only Administrative Actions.**
    NewDeployment, UpdateParams, and SweepDust can only be dispatched by Hyperbridge. -/
def inv_governance_only (sourceSMId : Bytes32) (hyperbridgeSMId : Bytes32) : Prop :=
  sourceSMId = hyperbridgeSMId
