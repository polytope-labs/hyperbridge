#!/bin/bash
set -e

declare -a testnet=("ethereum-sepolia" "arbitrum-sepolia" "optimism-sepolia" "base-sepolia" "bsc-testnet" "gnosis-chiado")
declare -a mainnet=("ethereum" "arbitrum" "optimism" "base" "bsc" "gnosis")

if [ "$2" == "mainnet" ]; then
   for i in "${mainnet[@]}"; do
      "$(pwd)/script/deploy.sh" "$i" $1 mainnet
   done
else
   for i in "${testnet[@]}"; do
      "$(pwd)/script/deploy.sh" "$i" $1 testnet
   done
fi
