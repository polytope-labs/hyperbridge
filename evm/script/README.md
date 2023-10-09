# How to deploy

Ensure you have a local beacon chain testnet running, see [polytope-labs/eth-pos-devnet](https://github.com/polytope-labs/eth-pos-devnet).

Fill out an `.env` file at the root of this repo with the given contents.

```dotenv
export ADMIN=0x123463a4B065722E99115D6c222f267d9cABb524
export PARA_ID=2000
export GOERLI_RPC_URL=http://127.0.0.1:8545
export PRIVATE_KEY=2e0834786285daccd064ca17f1654f67b4aef298acbb82cef9ec422fb4975622
```

The given private key is for the prefunded `0x123463a4B065722E99115D6c222f267d9cABb524` account in the devnet.

Run the command below to deploy

```shell
./scripts/deploy.sh {local|goerli|optimism-goerli|arbitrum-goerli|base-goerli}
```
