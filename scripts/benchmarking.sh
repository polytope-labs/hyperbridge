#!/bin/bash


declare -a arr=(
"frame_system"
"pallet_balances"
"pallet_timestamp"
"cumulus_pallet_xcmp_queue"
"pallet_message_queue"
"pallet_sudo"
"pallet_assets"
"pallet_utility"
"cumulus_pallet_parachain_system"
"pallet_session"
"ismp_grandpa"
)

# nexus runtime
for i in "${arr[@]}"
do
    cargo run -F=runtime-benchmarks -rp hyperbridge benchmark pallet \
        --wasm-execution=compiled \
        --pallet="$i" \
        --extrinsic="*" \
        --steps=50 \
        --repeat=20 \
        --genesis-builder=runtime \
        --runtime=./target/release/wbuild/nexus-runtime/nexus_runtime.compact.wasm \
        --output "parachain/runtimes/nexus/src/weights/$i.rs"
done

# gargantua runtime
for i in "${arr[@]}"
do
    cargo run -F=runtime-benchmarks -rp hyperbridge benchmark pallet \
        --wasm-execution=compiled \
        --pallet="$i" \
        --extrinsic="*" \
        --steps=50 \
        --repeat=20 \
        --genesis-builder=runtime \
        --runtime=./target/release/wbuild/gargantua-runtime/gargantua_runtime.compact.wasm \
        --output="parachain/runtimes/gargantua/src/weights/$i.rs"
done
