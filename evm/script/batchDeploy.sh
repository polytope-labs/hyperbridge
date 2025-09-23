#!/bin/bash
set -e
set -o xtrace

declare -a testnet=("SEPOLIA" "ARBITRUM_SEPOLIA" "OPTIMISM_SEPOLIA" "BASE_SEPOLIA" "BSC_TESTNET" "GNOSIS_CHIADO")
declare -a mainnet=("ETHEREUM" "ARBITRUM" "OPTIMISM" "BASE" "BNB" "GNOSIS" "POLYGON" "UNICHAIN")

if [ "$2" == "mainnet" ]; then
   for i in "${mainnet[@]}"; do
      "$(pwd)/script/deploy.sh" "$i" $1 mainnet
   done
else
   for i in "${testnet[@]}"; do
      "$(pwd)/script/deploy.sh" "$i" $1 testnet
   done
fi
