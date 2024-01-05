# smh we need to ingore plonk because our statically linked foundry lib can't compile it
mv ./evm/src/beefy/PlonkVerifier.sol ./evm/src/beefy/PlonkVerifier
mv ./evm/src/beefy/ZkBeefy.sol ./evm/src/beefy/ZkBeefy
mv ./evm/test/PlonkTest.sol ./evm/test/PlonkTest

eval $1

mv ./evm/src/beefy/PlonkVerifier ./evm/src/beefy/PlonkVerifier.sol
mv ./evm/src/beefy/ZkBeefy ./evm/src/beefy/ZkBeefy.sol
mv ./evm/test/PlonkTest ./evm/test/PlonkTest.sol