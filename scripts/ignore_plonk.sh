#!/bin/bash

set -e

# smh we need to ingore plonk because our statically linked foundry lib can't compile it
mv ./evm/src/beefy/verifiers/KusamaVerifier.sol ./evm/src/beefy/verifiers/KusamaVerifier
mv ./evm/src/beefy/verifiers/PolkadotVerifier.sol ./evm/src/beefy/verifiers/PolkadotVerifier
mv ./evm/src/beefy/verifiers/RococoVerifier.sol ./evm/src/beefy/verifiers/RococoVerifier
mv ./evm/test/PlonkTest.sol ./evm/test/PlonkTest
mv ./evm/test/ZkBeefyTest.sol ./evm/test/ZkBeefyTest

eval $1

mv ./evm/src/beefy/verifiers/KusamaVerifier ./evm/src/beefy/verifiers/KusamaVerifier.sol
mv ./evm/src/beefy/verifiers/PolkadotVerifier ./evm/src/beefy/verifiers/PolkadotVerifier.sol
mv ./evm/src/beefy/verifiers/RococoVerifier ./evm/src/beefy/verifiers/RococoVerifier.sol
mv ./evm/test/PlonkTest ./evm/test/PlonkTest.sol
mv ./evm/test/ZkBeefyTest ./evm/test/ZkBeefyTest.sol
