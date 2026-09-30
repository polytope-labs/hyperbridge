# Chain verification results stay with the chain

RPC and bundler verification no longer reports through a temporary page-level toast. The setup wizard and operator chain settings show checking, success, warning, or error feedback beside the selected chain's Verify button. That result remains visible while the operator works on other networks and clears when any RPC or bundler endpoint in its own row changes, so a stale success cannot describe edited connection details.

Files: `ui/src/components/EndpointVerificationStatus.tsx`, `ui/src/{wizard/steps,operator}/Chains.tsx`, `ui/src/operator/chains/useChainSettings.ts`, `ui/src/wizard/state.ts`, and `ui/src/styles/setup-controls.css`.
