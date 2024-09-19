#!/bin/bash

cargo release \
-p serde-hex-utils \
-p ismp \
-p mmr-primitives \
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
--execute
