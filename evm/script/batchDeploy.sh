#!/bin/bash

declare -a arr=("sepolia" "arbitrum-sepolia" "optimism-sepolia" "base-sepolia" "bsc-testnet")

for i in "${arr[@]}"
do
   "$(pwd)/script/deploy.sh" "$i" $1
done