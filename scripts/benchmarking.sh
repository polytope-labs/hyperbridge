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
)

# nexus runtime
for i in "${arr[@]}"
do
    target/release/hyperbridge benchmark pallet \
        --chain=nexus-2000 \
        --wasm-execution=compiled \
        --pallet "$i" \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output "parachain/runtimes/nexus/src/weights/$i.rs"
done

# messier runtime
# for i in "${arr[@]}"
# do
#     target/release/hyperbridge benchmark pallet \
#         --chain=messier-2000 \
#         --wasm-execution=compiled \
#         --pallet "$i" \
#         --extrinsic "*" \
#         --steps 50 \
#         --repeat 20 \
#         --output "parachain/runtimes/messier/src/weights/$i.rs"
# done

# gargantua runtime
for i in "${arr[@]}"
do
    target/release/hyperbridge benchmark pallet \
        --chain=gargantua-2000 \
        --wasm-execution=compiled \
        --pallet "$i" \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output "parachain/runtimes/gargantua/src/weights/$i.rs"
done
