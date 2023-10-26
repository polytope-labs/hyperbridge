if [ "$1" = "local" ]; then
    echo "Deploying locally"
    # load local .env
    source .env
    # deploy
    HOST="ethereum" forge script script/DeployIsmp.s.sol:DeployScript --rpc-url "$GOERLI_RPC_URL" --broadcast --verify -vvvv --sender="$ADMIN"
else
    echo "Deploying to $1"
    # load prod .env
    source .env.prod
    # deploy
    HOST=$1 forge script script/DeployIsmp.s.sol:DeployScript --rpc-url "$1" --broadcast -vvvv --sender="$ADMIN"
    # verify
    HOST=$1 forge script script/DeployIsmp.s.sol:DeployScript --rpc-url "$1" --resume --verify -vvvv --sender="$ADMIN"
fi
