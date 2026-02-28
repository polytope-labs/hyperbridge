/-
  Copyright (C) Polytope Labs Ltd.
  SPDX-License-Identifier: Apache-2.0

  Formal specification of IntentGatewayV2 contract state.
  Models the storage layout and per-chain gateway state.
-/
import Types

/-- Order status in the lifecycle. -/
inductive OrderStatus where
  | Open            -- order placed, escrow held, not yet filled
  | PartiallyFilled -- some outputs delivered (same-chain only)
  | Filled          -- fully filled by a solver
  | Refunded        -- cancelled and escrow returned to user
  deriving Repr, BEq, DecidableEq

/-- Escrow entry for a single token under a commitment. -/
structure EscrowEntry where
  token  : Address
  amount : UInt256
  deriving Repr, BEq, DecidableEq

/-- Composite key for escrow and partial fill mappings. -/
structure CommitmentTokenKey where
  commitment : Bytes32
  token      : Bytes32
  deriving Repr, BEq, DecidableEq, Hashable

instance : BEq CommitmentTokenKey where
  beq a b := a.commitment == b.commitment && a.token == b.token

/-- The on-chain state of a single IntentGatewayV2 instance.
    Models all storage variables from the contract. -/
structure GatewayState where
  /-- `mapping(bytes32 => address) _filled` -/
  filled : Bytes32 → Option Address
  /-- `uint256 _nonce` -/
  nonce : UInt256
  /-- `Params _params` -/
  params : Params
  /-- `address _admin` (zero after initialization) -/
  admin : Address
  /-- `mapping(bytes32 => mapping(address => uint256)) _orders`
      Models escrow balances per (commitment, token). -/
  orders : Bytes32 → Address → UInt256
  /-- `mapping(bytes32 => address) _instances`
      Maps hash(stateMachineId) → gateway address. -/
  instances : Bytes32 → Option Address
  /-- `mapping(bytes32 => mapping(bytes32 => uint256)) _partialFills`
      Cumulative fill progress per (commitment, outputToken). -/
  partialFills : Bytes32 → Bytes32 → UInt256
  /-- `mapping(bytes32 => uint256) _destinationProtocolFees`
      Per-destination fee overrides in basis points. -/
  destinationProtocolFees : Bytes32 → UInt256
  /-- The chain this gateway is deployed on (hash of host().host()). -/
  currentChain : Bytes32
  /-- Current block number (for deadline checks). -/
  blockNumber : UInt256

/-- Default empty gateway state for specification convenience. -/
def GatewayState.empty (chain : Bytes32) (block : UInt256) : GatewayState :=
  { filled := fun _ => none
    nonce := 0
    params := { host := 0, dispatcher := 0, solverSelection := false
                surplusShareBps := 0, protocolFeeBps := 0, priceOracle := 0 }
    admin := 0
    orders := fun _ _ => 0
    instances := fun _ => none
    partialFills := fun _ _ => 0
    destinationProtocolFees := fun _ => 0
    currentChain := chain
    blockNumber := block }

/-- Resolve gateway instance for a state machine.
    Models: `_instance(stateMachineId)` which falls back to self (0 here). -/
def GatewayState.instance (s : GatewayState) (smHash : Bytes32) : Address :=
  match s.instances smHash with
  | some addr => addr
  | none => 0  -- address(this) sentinel

/-- Resolve effective protocol fee for a destination.
    Uses destination-specific override if nonzero, else global fee. -/
def GatewayState.effectiveProtocolFee (s : GatewayState) (destHash : Bytes32) : UInt256 :=
  let destFee := s.destinationProtocolFees destHash
  if destFee > 0 then destFee else s.params.protocolFeeBps

/-- Compute reduced input amount after protocol fee deduction. -/
def deductProtocolFee (amount : UInt256) (feeBps : UInt256) : UInt256 × UInt256 :=
  let fee := (amount * feeBps) / BPS_DENOMINATOR
  (amount - fee, fee)

/-- Compute surplus split between protocol and beneficiary.
    When calldata is present, all surplus goes to protocol.
    Otherwise split according to surplusShareBps. -/
def splitSurplus (surplus : UInt256) (surplusShareBps : UInt256) (hasCalldata : Bool) : UInt256 × UInt256 :=
  if hasCalldata then
    (surplus, 0)  -- (protocolShare, beneficiaryShare)
  else
    let protocolShare := (surplus * surplusShareBps) / BPS_DENOMINATOR
    (protocolShare, surplus - protocolShare)

/-- Total escrowed amount for a commitment across a list of tokens. -/
def totalEscrowed (s : GatewayState) (commitment : Bytes32) (tokens : List TokenInfo) : UInt256 :=
  tokens.foldl (fun acc ti => acc + s.orders commitment (ti.token)) 0
