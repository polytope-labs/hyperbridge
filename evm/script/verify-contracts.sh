#!/bin/bash

# Load environment
source .env.testnet

# Deployed addresses (same on both chains due to CREATE2 with same salt)
CALL_DISPATCHER="0x876F1891982E260026630c233A4897160A281Fb8"
VWAP_ORACLE="0xe73b283506eD9F86AA11C0086B5052B4E6Fa1686"
INTENT_GATEWAY_V2="0xFbF50B2b32768127603cC9eF4b871574b881b8eD"
SOLVER_ACCOUNT="0xd42EFC09607dA5577dfB7Ecc3E0756b0f45902E3"

# Chain ID: 97 = BSC Testnet, 80002 = Polygon Amoy
CHAIN_ID=${1:-97}

echo "=== Verifying contracts on Chain ID: $CHAIN_ID ==="

# 1. Verify CallDispatcher (no constructor args)
echo "Verifying CallDispatcher..."
forge verify-contract \
  --chain-id $CHAIN_ID \
  --watch \
  $CALL_DISPATCHER \
  src/utils/CallDispatcher.sol:CallDispatcher

# 2. Verify VWAPOracle (constructor arg: admin)
echo "Verifying VWAPOracle..."
forge verify-contract \
  --chain-id $CHAIN_ID \
  --watch \
  --constructor-args $(cast abi-encode "constructor(address)" $ADMIN) \
  $VWAP_ORACLE \
  src/utils/VWAPOracle.sol:VWAPOracle

# 3. Verify IntentGatewayV2 (constructor arg: admin)
echo "Verifying IntentGatewayV2..."
forge verify-contract \
  --chain-id $CHAIN_ID \
  --watch \
  --constructor-args $(cast abi-encode "constructor(address)" $ADMIN) \
  $INTENT_GATEWAY_V2 \
  src/apps/IntentGatewayV2.sol:IntentGatewayV2

# 4. Verify SolverAccount (constructor arg: intentGatewayV2)
echo "Verifying SolverAccount..."
forge verify-contract \
  --chain-id $CHAIN_ID \
  --watch \
  --constructor-args $(cast abi-encode "constructor(address)" $INTENT_GATEWAY_V2) \
  $SOLVER_ACCOUNT \
  src/utils/SolverAccount.sol:SolverAccount

echo "=== Verification complete ==="
