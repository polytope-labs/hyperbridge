#!/bin/bash

set -euo pipefail

PACKAGE="$1"

# Gargantua release builds turn on the negatively-biased `no-bandwidth`
# flag so the `pallet-bandwidth` pallet is stripped from the deployed
# testnet runtime. The runtime keeps bandwidth on by default for local
# development; only release builds opt out.
if [ "$PACKAGE" = "gargantua-runtime" ]; then
    cargo build -p "$PACKAGE" --features=metadata-hash,no-bandwidth --release
else
    cargo build -p "$PACKAGE" --features=metadata-hash --release
fi
