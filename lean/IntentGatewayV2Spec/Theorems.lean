/-
  Copyright (C) Polytope Labs Ltd.
  SPDX-License-Identifier: Apache-2.0

  Formal theorems and proofs for IntentGatewayV2 invariants.
  Each theorem proves that a specific invariant is preserved by
  the corresponding state transition.
-/
import Invariants

-- ============================================================================
-- Theorem 1: Fill Finality — filling an order marks it as filled
-- ============================================================================

theorem fill_marks_commitment_same_chain
    (s : GatewayState) (order : Order) (solver : Address) (commitment : Bytes32) :
    let s' := fillSameChain_full_transition s order solver commitment
    s'.filled commitment = some solver := by
  simp [fillSameChain_full_transition, updateFn]

theorem fill_marks_commitment_cross_chain
    (s : GatewayState) (solver : Address) (commitment : Bytes32) :
    let s' := fillCrossChain_transition s solver commitment
    s'.filled commitment = some solver := by
  simp [fillCrossChain_transition, updateFn]

-- ============================================================================
-- Theorem 2: No Double Fill — filled orders cannot be re-filled
-- ============================================================================

theorem no_double_fill_same_chain
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillSameChain_pre s order options commitment) :
    s.filled commitment = none := by
  exact h_pre.2.1

theorem no_double_fill_cross_chain
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillCrossChain_pre s order options commitment) :
    s.filled commitment = none := by
  exact h_pre.2.1

-- ============================================================================
-- Theorem 3: Fill preserves other commitments
-- ============================================================================

theorem fill_same_chain_preserves_other_commitments
    (s : GatewayState) (order : Order) (solver : Address)
    (commitment other : Bytes32) (h_ne : other ≠ commitment) :
    let s' := fillSameChain_full_transition s order solver commitment
    s'.filled other = s.filled other := by
  simp [fillSameChain_full_transition, updateFn, h_ne]

theorem fill_cross_chain_preserves_other_commitments
    (s : GatewayState) (solver : Address)
    (commitment other : Bytes32) (h_ne : other ≠ commitment) :
    let s' := fillCrossChain_transition s solver commitment
    s'.filled other = s.filled other := by
  simp [fillCrossChain_transition, updateFn, h_ne]

-- ============================================================================
-- Theorem 4: Nonce Monotonicity — placeOrder increments nonce
-- ============================================================================

theorem place_order_increments_nonce (s : GatewayState) (order : Order) :
    let s' := placeOrder_transition s order
    s'.nonce = s.nonce + 1 := by
  simp [placeOrder_transition]

theorem place_order_nonce_monotonic (s : GatewayState) (order : Order) :
    let s' := placeOrder_transition s order
    inv_nonce_monotonic s s' := by
  simp [placeOrder_transition, inv_nonce_monotonic]

-- ============================================================================
-- Theorem 5: Admin One-Shot — setParams burns the admin
-- ============================================================================

theorem set_params_burns_admin (s : GatewayState) (p : Params) :
    let s' := setParams_transition s p
    s'.admin = 0 := by
  simp [setParams_transition]

theorem set_params_prevents_future_calls (s : GatewayState) (p : Params) :
    let s' := setParams_transition s p
    inv_admin_one_shot s' := by
  simp [setParams_transition, inv_admin_one_shot, setParams_pre]

-- ============================================================================
-- Theorem 6: Cancel Same-Chain marks as filled (preventing re-fill)
-- ============================================================================

theorem cancel_same_chain_marks_filled
    (s : GatewayState) (order : Order) (commitment : Bytes32) :
    let s' := cancelSameChain_transition s order commitment
    s'.filled commitment = some order.user := by
  simp [cancelSameChain_transition, updateFn]

-- ============================================================================
-- Theorem 7: Cancel from Destination marks as filled
-- ============================================================================

theorem cancel_from_dest_marks_filled
    (s : GatewayState) (order : Order) (commitment : Bytes32) :
    let s' := cancelFromDest_transition s order commitment
    s'.filled commitment = some order.user := by
  simp [cancelFromDest_transition, updateFn]

-- ============================================================================
-- Theorem 8: Deadline enforcement is a precondition of fill
-- ============================================================================

theorem fill_requires_valid_deadline_same_chain
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillSameChain_pre s order options commitment) :
    inv_deadline_fill s order := by
  exact h_pre.1

theorem fill_requires_valid_deadline_cross_chain
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillCrossChain_pre s order options commitment) :
    inv_deadline_fill s order := by
  exact h_pre.1

-- ============================================================================
-- Theorem 9: Cancel-from-source requires expiry
-- ============================================================================

theorem cancel_source_requires_expiry
    (s : GatewayState) (order : Order) (caller : Address) (options : CancelOptions)
    (commitment : Bytes32)
    (h_pre : cancelFromSource_pre s order caller options commitment) :
    inv_deadline_cancel_source order options := by
  exact h_pre.2.2.1

-- ============================================================================
-- Theorem 10: Cross-chain fill is all-or-nothing
-- ============================================================================

theorem cross_chain_fill_all_or_nothing
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillCrossChain_pre s order options commitment) :
    inv_cross_chain_all_or_nothing options order := by
  exact h_pre.2.2.2.2.2.2

-- ============================================================================
-- Theorem 11: Chain routing correctness for same-chain fill
-- ============================================================================

theorem same_chain_fill_routing
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillSameChain_pre s order options commitment) :
    chainHash order.source = s.currentChain ∧
    chainHash order.destination = s.currentChain := by
  constructor
  · exact h_pre.2.2.2.2.2
  · have h1 := h_pre.2.2.2.2.1
    have h2 := h_pre.2.2.2.2.2
    rw [h1] at h2
    exact h2

theorem cross_chain_fill_routing
    (s : GatewayState) (order : Order) (options : FillOptions) (commitment : Bytes32)
    (h_pre : fillCrossChain_pre s order options commitment) :
    chainHash order.destination = s.currentChain := by
  exact h_pre.2.2.2.2.2.1

-- ============================================================================
-- Lemma: division by denominator is bounded when numerator factors are bounded
-- ============================================================================

private theorem div_le_when_factor_le (n factor denom : Nat) (hd : denom > 0) (hf : factor ≤ denom) :
    (n * factor) / denom ≤ n := by
  rw [Nat.div_le_iff_le_mul hd]
  have h1 : n * factor ≤ n * denom :=
    Nat.mul_le_mul_left n hf
  omega

-- ============================================================================
-- Theorem 12: Protocol fee deduction is conservative
-- ============================================================================

theorem protocol_fee_deduction_conservative (amount feeBps : UInt256)
    (h_bound : feeBps < BPS_DENOMINATOR) :
    let fee := (amount * feeBps) / BPS_DENOMINATOR
    fee ≤ amount := by
  simp only
  exact div_le_when_factor_le amount feeBps BPS_DENOMINATOR (by simp [BPS_DENOMINATOR]) (Nat.le_of_lt h_bound)

theorem protocol_fee_reduced_leq_original (amount feeBps : UInt256) :
    let (reduced, _) := deductProtocolFee amount feeBps
    reduced ≤ amount := by
  simp only [deductProtocolFee]
  exact Nat.sub_le amount _

-- ============================================================================
-- Theorem 13: Deployment registration updates instances
-- ============================================================================

theorem add_deployment_registers_instance
    (s : GatewayState) (deploy : NewDeployment) :
    let s' := addDeployment_transition s deploy
    let smHash := chainHash deploy.stateMachineId
    s'.instances smHash = some deploy.gateway := by
  simp [addDeployment_transition, updateFn]

-- ============================================================================
-- Theorem 14: Params update applies new params
-- ============================================================================

theorem update_params_applies_new_params
    (s : GatewayState) (update : ParamsUpdate) :
    let s' := updateParams_transition s update
    s'.params = update.params := by
  simp [updateParams_transition]

-- ============================================================================
-- Theorem 15: Cancel authorization — same-chain requires ownership
-- ============================================================================

theorem cancel_same_chain_requires_ownership
    (s : GatewayState) (order : Order) (caller : Address) (commitment : Bytes32)
    (h_pre : cancelSameChain_pre s order caller commitment) :
    order.user = caller := by
  exact h_pre.2.1

-- ============================================================================
-- Theorem 16: Redeem escrow marks commitment as filled
-- ============================================================================

theorem redeem_escrow_marks_filled
    (s : GatewayState) (req : WithdrawalRequest) :
    let s' := redeemEscrow_transition s req
    s'.filled req.commitment = some req.beneficiary := by
  simp [redeemEscrow_transition, updateFn]

-- ============================================================================
-- Theorem 17: Refund escrow marks commitment as filled (preventing re-refund)
-- ============================================================================

theorem refund_escrow_marks_filled
    (s : GatewayState) (req : WithdrawalRequest) :
    let s' := refundEscrow_transition s req
    s'.filled req.commitment = some req.beneficiary := by
  simp [refundEscrow_transition, updateFn]

-- ============================================================================
-- Theorem 18: placeOrder does not affect filled mapping
-- ============================================================================

theorem place_order_preserves_filled
    (s : GatewayState) (order : Order) (commitment : Bytes32) :
    let s' := placeOrder_transition s order
    s'.filled commitment = s.filled commitment := by
  simp [placeOrder_transition]

-- ============================================================================
-- Theorem 19: Surplus split is conservative (no value created/destroyed)
-- ============================================================================

theorem surplus_split_conservative_with_calldata (surplus surplusShareBps : UInt256) :
    let (protocolShare, beneficiaryShare) := splitSurplus surplus surplusShareBps true
    protocolShare + beneficiaryShare = surplus := by
  simp [splitSurplus]

theorem surplus_split_conservative_without_calldata (surplus surplusShareBps : UInt256)
    (h_bound : surplusShareBps ≤ BPS_DENOMINATOR) :
    let (protocolShare, beneficiaryShare) := splitSurplus surplus surplusShareBps false
    protocolShare + beneficiaryShare = surplus := by
  simp only [splitSurplus, ite_false]
  exact Nat.add_sub_cancel'
    (div_le_when_factor_le surplus surplusShareBps BPS_DENOMINATOR (by simp [BPS_DENOMINATOR]) h_bound)

-- ============================================================================
-- Theorem 20: Fill and cancel are mutually exclusive on same commitment
-- ============================================================================

theorem fill_and_cancel_mutually_exclusive
    (s : GatewayState) (commitment : Bytes32)
    (h_filled : s.filled commitment ≠ none) :
    ¬ (∃ order options, fillSameChain_pre s order options commitment) ∧
    ¬ (∃ order caller, cancelSameChain_pre s order caller commitment) := by
  constructor
  · intro ⟨_, _, h_pre⟩
    exact h_filled h_pre.2.1
  · intro ⟨_, _, h_pre⟩
    exact h_filled h_pre.1
