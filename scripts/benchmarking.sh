#!/bin/bash

set -e

declare -a nexus=(
"cumulus_pallet_parachain_system"
"cumulus_pallet_xcmp_queue"
"frame_system"
"ismp_grandpa"
"ismp_parachain"
"pallet_asset_rate"
"pallet_assets"
"pallet_balances"
"pallet_collator_selection"
"pallet_collective"
"pallet_message_queue"
"pallet_multisig"
"pallet_proxy"
"pallet_session"
"pallet_sudo"
"pallet_timestamp"
# "pallet_treasury" broken on stable2409
"pallet_utility"
)

cargo build -rp hyperbridge -F=runtime-benchmarks

# nexus runtime
for i in "${nexus[@]}"
do
    target/release/hyperbridge benchmark pallet \
        --wasm-execution=compiled \
        --pallet="$i" \
        --extrinsic="*" \
        --steps=50 \
        --repeat=20 \
        --unsafe-overwrite-results \
        --genesis-builder-preset=development \
        --template=./scripts/template.hbs \
        --genesis-builder=runtime \
        --runtime=./target/release/wbuild/nexus-runtime/nexus_runtime.compact.wasm \
        --output "parachain/runtimes/nexus/src/weights/$i.rs"
done

declare -a gargantua=(
"cumulus_pallet_parachain_system"
"cumulus_pallet_xcmp_queue"
"frame_system"
"ismp_grandpa"
"ismp_parachain"
"pallet_asset_rate"
"pallet_assets"
"pallet_balances"
"pallet_collective"
"pallet_message_queue"
"pallet_session"
"pallet_sudo"
"pallet_timestamp"
# "pallet_treasury" broken on stable2409
"pallet_utility"
)

# gargantua runtime
for i in "${gargantua[@]}"
do
    target/release/hyperbridge benchmark pallet \
        --wasm-execution=compiled \
        --pallet="$i" \
        --extrinsic="*" \
        --steps=50 \
        --repeat=20 \
        --unsafe-overwrite-results \
        --genesis-builder-preset=development \
        --template=./scripts/template.hbs \
        --genesis-builder=runtime \
        --runtime=./target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.wasm \
        --output="parachain/runtimes/gargantua/src/weights/$i.rs"
done
