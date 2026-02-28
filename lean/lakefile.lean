import Lake
open Lake DSL

package IntentGatewayV2Spec where
  leanOptions := #[
    ⟨`autoImplicit, false⟩
  ]

@[default_target]
lean_lib IntentGatewayV2Spec where
  srcDir := "IntentGatewayV2Spec"
  roots := #[`Types, `State, `Transitions, `Invariants, `Theorems]
