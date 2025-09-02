#!/bin/bash

cargo release \
-p serde-hex-utils \
-p crypto-utils \
-p ismp \
-p pallet-hyperbridge \
-p pallet-ismp \
-p pallet-ismp-runtime-api \
-p pallet-ismp-rpc \
-p substrate-state-machine \
-p ismp-parachain \
-p grandpa-verifier-primitives \
-p grandpa-verifier \
-p ismp-grandpa \
-p ismp-parachain-runtime-api \
-p ismp-parachain-inherent \
-p token-gateway-primitives \
-p pallet-token-gateway \
--execute
