#!/bin/bash

echo "================Deploying to $1, environment: $3 ================"
# load prod .env
source "$(pwd)/.env.$3"
# remove existing sources
rm -rf out/ cache/ broadcast/
# deploy
HOST=$1 forge script "script/Deploy$2.s.sol:DeployScript" --rpc-url "${1,,}" -vvv --sender="$ADMIN" --broadcast
# HOST=$1 forge script "script/Deploy$2.s.sol:DeployScript" --rpc-url "${1,,}" -vvv --sender="$ADMIN" 
# verify
HOST=$1 forge script "script/Deploy$2.s.sol:DeployScript" --rpc-url "${1,,}" --resume --verify -vvvvv --private-keys $PRIVATE_KEY
