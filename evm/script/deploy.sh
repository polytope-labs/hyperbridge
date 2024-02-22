#!/bin/bash
if [ "$1" = "local" ]; then
    echo "Deploying locally"
    # load local .env
    source "$(pwd)/.env"
    # deploy
    HOST="ethereum" forge script script/DeployIsmp.s.sol:DeployScript --rpc-url "$GOERLI_RPC_URL" --broadcast -vvvvv --sender="$ADMIN"
else
    echo "Deploying to $1"
    # load prod .env
    source "$(pwd)/.env.prod"
    # remove existing sources
    rm -rf out/ cache/ broadcast/
    # deploy
    HOST=$1 forge script "script/Deploy$2.s.sol:DeployScript" --rpc-url "$1" --broadcast -vvvv --sender="$ADMIN"
    # verify
    HOST=$1 forge script "script/Deploy$2.s.sol:DeployScript" --rpc-url "$1" --resume --verify -vvvvv --sender="$ADMIN"
fi
