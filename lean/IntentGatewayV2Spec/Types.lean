/-
  Copyright (C) Polytope Labs Ltd.
  SPDX-License-Identifier: Apache-2.0

  Formal specification of IntentGatewayV2 data types.
  Models the Solidity structs used by the contract.
-/

/-- 256-bit unsigned integer (models Solidity uint256). -/
abbrev UInt256 := Nat

/-- 160-bit address (models Solidity address). -/
abbrev Address := Nat

/-- 256-bit hash (models Solidity bytes32). -/
abbrev Bytes32 := Nat

/-- Basis points constant: 10000 = 100%. -/
def BPS_DENOMINATOR : Nat := 10000

/-- Token identifier and amount pair.
    Models: `struct TokenInfo { bytes32 token; uint256 amount; }` -/
structure TokenInfo where
  token : Bytes32
  amount : UInt256
  deriving Repr, BEq, DecidableEq

/-- Payment output specification with beneficiary and optional calldata.
    Models: `struct PaymentInfo { bytes32 beneficiary; TokenInfo[] assets; bytes call; }` -/
structure PaymentInfo where
  beneficiary : Bytes32
  assets : List TokenInfo
  call : List UInt8  -- empty list = no calldata
  deriving Repr, BEq, DecidableEq

/-- Pre-dispatch call information with assets and calldata.
    Models: `struct DispatchInfo { TokenInfo[] assets; bytes call; }` -/
structure DispatchInfo where
  assets : List TokenInfo
  call : List UInt8
  deriving Repr, BEq, DecidableEq

/-- An intent order.
    Models the Solidity `Order` struct exactly. -/
structure Order where
  user        : Bytes32
  source      : List UInt8     -- state machine identifier bytes
  destination : List UInt8
  deadline    : UInt256        -- block number deadline
  nonce       : UInt256
  fees        : UInt256        -- relayer fee amount
  session     : Address        -- session key for solver selection
  predispatch : DispatchInfo
  inputs      : List TokenInfo -- tokens escrowed on source chain
  output      : PaymentInfo    -- desired output on destination chain
  deriving Repr, BEq, DecidableEq

/-- Gateway configuration parameters.
    Models: `struct Params` -/
structure Params where
  host            : Address
  dispatcher      : Address
  solverSelection : Bool
  surplusShareBps : UInt256  -- protocol's share of surplus in basis points
  protocolFeeBps  : UInt256  -- protocol fee on inputs in basis points
  priceOracle     : Address
  deriving Repr, BEq, DecidableEq

/-- Destination-specific fee override.
    Models: `struct DestinationFee` -/
structure DestinationFee where
  destinationFeeBps : UInt256
  stateMachineId    : Bytes32
  deriving Repr, BEq, DecidableEq

/-- Parameter update request from governance.
    Models: `struct ParamsUpdate` -/
structure ParamsUpdate where
  params          : Params
  destinationFees : List DestinationFee
  deriving Repr, BEq, DecidableEq

/-- Withdrawal/refund request.
    Models: `struct WithdrawalRequest` -/
structure WithdrawalRequest where
  commitment  : Bytes32
  beneficiary : Bytes32
  tokens      : List TokenInfo
  deriving Repr, BEq, DecidableEq

/-- Fill options provided by solver.
    Models: `struct FillOptions` -/
structure FillOptions where
  relayerFee       : UInt256
  nativeDispatchFee : UInt256
  outputs          : List TokenInfo
  deriving Repr, BEq, DecidableEq

/-- Solver selection options.
    Models: `struct SelectOptions` -/
structure SelectOptions where
  commitment : Bytes32
  solver     : Address
  signature  : List UInt8
  deriving Repr, BEq, DecidableEq

/-- Cancel options.
    Models: `struct CancelOptions` -/
structure CancelOptions where
  relayerFee : UInt256
  height     : UInt256
  deriving Repr, BEq, DecidableEq

/-- Cross-chain request discriminator.
    Models: `enum RequestKind` -/
inductive RequestKind where
  | RedeemEscrow
  | NewDeployment
  | UpdateParams
  | SweepDust
  | RefundEscrow
  deriving Repr, BEq, DecidableEq

/-- Dust sweep request from governance.
    Models: `struct SweepDust` -/
structure SweepDust where
  beneficiary : Address
  outputs     : List TokenInfo
  deriving Repr, BEq, DecidableEq

/-- New gateway deployment registration.
    Models: `struct NewDeployment` -/
structure NewDeployment where
  stateMachineId : List UInt8
  gateway        : Address
  deriving Repr, BEq, DecidableEq

/-- Commitment hash computation (abstract model).
    In the real contract this is `keccak256(abi.encode(order))`. -/
opaque orderCommitment : Order → Bytes32

/-- Chain identifier hash (abstract model of `keccak256(chainId)`). -/
opaque chainHash : List UInt8 → Bytes32

/-- Pairwise relation on two lists (replaces Mathlib's List.Forall₂). -/
inductive Pairwise {α : Type} {β : Type} (R : α → β → Prop) : List α → List β → Prop where
  | nil  : Pairwise R [] []
  | cons : {a : α} → {b : β} → {as : List α} → {bs : List β} →
           R a b → Pairwise R as bs → Pairwise R (a :: as) (b :: bs)
