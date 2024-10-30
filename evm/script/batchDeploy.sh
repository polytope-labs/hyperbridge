#!/bin/bash

declare -a arr=("ethereum-sepolia" "arbitrum-sepolia" "optimism-sepolia" "base-sepolia" "bsc-testnet" "gnosis-chiado")

for i in "${arr[@]}"
do
   "$(pwd)/script/deploy.sh" "$i" $1 $2
done
