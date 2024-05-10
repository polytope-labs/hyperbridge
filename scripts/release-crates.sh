#!/bin/bash

cargo release \
-p ismp \
-p mmr-primitives \
-p pallet-hyperbridge \
-p pallet-ismp \
-p pallet-ismp-runtime-api \
-p pallet-ismp-rpc \
-p substrate-state-machine \
-p ismp-parachain \
-p ismp-parachain-runtime-api \
-p ismp-parachain-inherent \
--execute
