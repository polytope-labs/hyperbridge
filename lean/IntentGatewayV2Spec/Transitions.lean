/-
  Copyright (C) Polytope Labs Ltd.
  SPDX-License-Identifier: Apache-2.0

  Formal specification of IntentGatewayV2 state transitions.
  Each transition models a public/external function on the contract.
-/
import State

-- ============================================================================
-- Preconditions
-- ============================================================================

/-- Precondition for `placeOrder`: the order has at least one input. -/
def placeOrder_pre (order : Order) : Prop :=
  order.inputs.length > 0 ∧
  order.inputs.all (fun ti => ti.amount > 0)

/-- Precondition for `fillOrder` on same-chain: the order is valid and fillable. -/
def fillSameChain_pre (s : GatewayState) (order : Order) (options : FillOptions)
    (commitment : Bytes32) : Prop :=
  order.deadline ≥ s.blockNumber ∧
  s.filled commitment = none ∧
  options.outputs.length = order.output.assets.length ∧
  order.inputs.length = order.output.assets.length ∧
  chainHash order.source = chainHash order.destination ∧
  chainHash order.source = s.currentChain

/-- Precondition for `fillOrder` on cross-chain: the order is valid and fillable. -/
def fillCrossChain_pre (s : GatewayState) (order : Order) (options : FillOptions)
    (commitment : Bytes32) : Prop :=
  order.deadline ≥ s.blockNumber ∧
  s.filled commitment = none ∧
  options.outputs.length = order.output.assets.length ∧
  order.inputs.length = order.output.assets.length ∧
  chainHash order.source ≠ chainHash order.destination ∧
  chainHash order.destination = s.currentChain ∧
  -- cross-chain fills are all-or-nothing: solver must provide ≥ required for each output
  Pairwise (fun opt req => opt.amount ≥ req.amount) options.outputs order.output.assets

/-- Precondition for same-chain cancel. -/
def cancelSameChain_pre (s : GatewayState) (order : Order) (caller : Address)
    (commitment : Bytes32) : Prop :=
  s.filled commitment = none ∧
  order.user = caller ∧
  chainHash order.source = chainHash order.destination ∧
  chainHash order.source = s.currentChain ∧
  -- at least one input has escrow
  order.inputs.any (fun ti => s.orders commitment ti.token > 0)

/-- Precondition for cross-chain cancel from source. -/
def cancelFromSource_pre (s : GatewayState) (order : Order) (caller : Address)
    (options : CancelOptions) (commitment : Bytes32) : Prop :=
  s.filled commitment = none ∧
  order.user = caller ∧
  options.height > order.deadline ∧
  chainHash order.source ≠ chainHash order.destination ∧
  chainHash order.source = s.currentChain ∧
  order.inputs.all (fun ti => s.orders commitment ti.token > 0)

/-- Precondition for cross-chain cancel from destination. -/
def cancelFromDest_pre (s : GatewayState) (order : Order) (caller : Address)
    (commitment : Bytes32) : Prop :=
  s.filled commitment = none ∧
  chainHash order.source ≠ chainHash order.destination ∧
  chainHash order.destination = s.currentChain ∧
  -- only owner can cancel before deadline
  (order.deadline ≥ s.blockNumber → order.user = caller)

/-- Precondition for `setParams`: caller must be admin and admin must be nonzero. -/
def setParams_pre (s : GatewayState) (caller : Address) : Prop :=
  s.admin ≠ 0 ∧ s.admin = caller

-- ============================================================================
-- State Transitions
-- ============================================================================

/-- Helper: update a function-based mapping at one key. -/
def updateFn {α β : Type} [DecidableEq α] (f : α → β) (k : α) (v : β) : α → β :=
  fun a => if a = k then v else f a

/-- Helper: update a nested function-based mapping at two keys. -/
def updateFn2 {α β γ : Type} [DecidableEq α] [DecidableEq β]
    (f : α → β → γ) (k1 : α) (k2 : β) (v : γ) : α → β → γ :=
  fun a b => if a = k1 && b = k2 then v else f a b

/-- Compute reduced inputs after protocol fee deduction. -/
def computeReducedInputs (inputs : List TokenInfo) (feeBps : UInt256) : List TokenInfo :=
  inputs.map fun ti =>
    let (reduced, _) := deductProtocolFee ti.amount feeBps
    { token := ti.token, amount := reduced }

/-- State transition for `placeOrder`.
    Escrows reduced input amounts and increments the nonce. -/
def placeOrder_transition (s : GatewayState) (order : Order) : GatewayState :=
  let destHash := chainHash order.destination
  let feeBps := s.effectiveProtocolFee destHash
  let reducedInputs := computeReducedInputs order.inputs feeBps
  let commitment := orderCommitment
    { order with
      user := order.user
      source := order.source
      nonce := s.nonce
      inputs := reducedInputs }
  let orders' := reducedInputs.foldl (fun acc ti =>
    let token := ti.token  -- using token as address proxy
    let prev := acc commitment token
    updateFn2 acc commitment token (prev + ti.amount)
  ) s.orders
  let fees_orders := if order.fees > 0 then
    -- TRANSACTION_FEES sentinel address = hash("txFees")
    let txFeeSentinel : Address := 0xDEAD  -- abstract sentinel
    updateFn2 orders' commitment txFeeSentinel order.fees
  else orders'
  { s with
    nonce := s.nonce + 1
    orders := fees_orders }

/-- State transition for a full same-chain fill.
    Releases all escrow to the solver and marks the order as filled. -/
def fillSameChain_full_transition (s : GatewayState) (order : Order) (solver : Address)
    (commitment : Bytes32) : GatewayState :=
  -- Release all escrowed inputs to solver
  let orders' := order.inputs.foldl (fun acc ti =>
    updateFn2 acc commitment ti.token 0
  ) s.orders
  { s with
    filled := updateFn s.filled commitment (some solver)
    orders := orders' }

/-- State transition for a cross-chain fill on the destination chain.
    Marks as filled on destination; escrow release happens via cross-chain message. -/
def fillCrossChain_transition (s : GatewayState) (solver : Address)
    (commitment : Bytes32) : GatewayState :=
  { s with
    filled := updateFn s.filled commitment (some solver) }

/-- State transition for same-chain cancel.
    Refunds all remaining escrow to the user and marks as filled (=refunded). -/
def cancelSameChain_transition (s : GatewayState) (order : Order) (commitment : Bytes32) : GatewayState :=
  let user := order.user  -- address extracted from bytes32
  let orders' := order.inputs.foldl (fun acc ti =>
    updateFn2 acc commitment ti.token 0
  ) s.orders
  { s with
    filled := updateFn s.filled commitment (some user)
    orders := orders' }

/-- State transition for cross-chain cancel from destination.
    Marks as filled locally to prevent future fills. -/
def cancelFromDest_transition (s : GatewayState) (order : Order) (commitment : Bytes32) : GatewayState :=
  { s with
    filled := updateFn s.filled commitment (some order.user) }

/-- State transition for escrow release via cross-chain callback (`onAccept` / RedeemEscrow).
    Releases escrowed tokens and marks the order as finalized. -/
def redeemEscrow_transition (s : GatewayState) (req : WithdrawalRequest) : GatewayState :=
  let orders' := req.tokens.foldl (fun acc ti =>
    let prev := acc req.commitment ti.token
    updateFn2 acc req.commitment ti.token (prev - ti.amount)
  ) s.orders
  { s with
    filled := updateFn s.filled req.commitment (some req.beneficiary)
    orders := orders' }

/-- State transition for escrow refund via cross-chain callback (`onAccept` / RefundEscrow). -/
def refundEscrow_transition (s : GatewayState) (req : WithdrawalRequest) : GatewayState :=
  let orders' := req.tokens.foldl (fun acc ti =>
    let prev := acc req.commitment ti.token
    updateFn2 acc req.commitment ti.token (prev - ti.amount)
  ) s.orders
  { s with
    filled := updateFn s.filled req.commitment (some req.beneficiary)
    orders := orders' }

/-- State transition for `setParams` (one-time admin initialization). -/
def setParams_transition (s : GatewayState) (p : Params) : GatewayState :=
  { s with
    params := p
    admin := 0 }

/-- State transition for adding a new deployment via governance. -/
def addDeployment_transition (s : GatewayState) (deploy : NewDeployment) : GatewayState :=
  let smHash := chainHash deploy.stateMachineId
  { s with
    instances := updateFn s.instances smHash (some deploy.gateway) }

/-- State transition for updating params via governance. -/
def updateParams_transition (s : GatewayState) (update : ParamsUpdate) : GatewayState :=
  let destFees' := update.destinationFees.foldl (fun acc df =>
    updateFn acc df.stateMachineId df.destinationFeeBps
  ) s.destinationProtocolFees
  { s with
    params := update.params
    destinationProtocolFees := destFees' }

/-- State transition for `onGetResponse` (cancel verification from source chain).
    If the filled slot is empty on the destination, refund the escrow. -/
def onGetResponse_refund_transition (s : GatewayState) (req : WithdrawalRequest) : GatewayState :=
  refundEscrow_transition s req
