#!/bin/bash

# EntryPoint v0.9 testnet rollout, EVM side, on BSC Chapel and Polygon Amoy.
# Usage: script/testnet/entrypoint-v09.sh [OPTIONS] <step> [step ...]
# The Hyperbridge governance steps run from sdk/packages/simplex/e2e/entrypoint-v09-sudo.mjs.
# README.md next to this script has the full order.

set -eo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

EVM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_DIR="$(cd "$EVM_DIR/.." && pwd)"
ENV_FILE="${ENV_FILE:-$REPO_DIR/sdk/.env.local}"
CONFIG="${CONFIG:-config.testnet.toml}"

ENTRYPOINT_V09=0x433709009B8330FDa32311DF1C2AFA402eD8D009
SELECT_SOLVER_TYPEHASH=0xe706bdab7d945360dcd9d81d355f856754dd1cfa461edfc0a7502e2583b4e09e
IMPLEMENTATION_SLOT=0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc
UNSTAKE_DELAY=86400

DRY_RUN=false
CHAINS="bsc-testnet,polygon-amoy"
DEPOSIT=""
STAKE=""
STEPS=()

print_usage() {
    echo -e "${BLUE}EntryPoint v0.9 testnet rollout (EVM steps)${NC}"
    echo ""
    echo "Usage: $0 [OPTIONS] <step> [step ...]"
    echo ""
    echo "Steps, each skipped on a chain where it already holds:"
    echo "  solver-account  1. CREATE2 SolverAccount; checks entryPoint() is v0.9 and one address on every chain"
    echo "  gateway-impl    2. IntentGatewayV2 implementation; prints the execute_on_gateway calldata"
    echo "  paymaster       3. fixed-price feeds, then a SimplexPaymaster proxy"
    echo "  fund            6. tops the paymaster's EntryPoint deposit and stake up to the targets"
    echo "  status          read-only report of every check above"
    echo ""
    echo "Options:"
    echo "  --dry-run           Run on local anvil forks with a temporary config copy; nothing leaves the machine"
    echo "  -c, --chains LIST   bsc-testnet,polygon-amoy (default: both)"
    echo "  --deposit ETHER     Deposit target (default: 0.3 on bsc-testnet, 1 on polygon-amoy)"
    echo "  --stake ETHER       Stake target (default: 0.05 on bsc-testnet, 0.1 on polygon-amoy)"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Environment (each falls back as listed; secrets are read from $ENV_FILE with grep, never sourced):"
    echo "  PRIVATE_KEY                 deployer key, else PRIVATE_KEY in the env file"
    echo "  BSC_TESTNET_RPC_URL         else BSC_CHAPEL in the env file"
    echo "  POLYGON_AMOY_RPC_URL        else POLYGON_AMOY in the env file"
    echo "  VERSION                     CREATE2 salt seed (default: entrypoint-v09)"
    echo "  GOVERNANCE_RELAYER          paymaster relayer (default: 0xc8809DD0b00370be097382d741A43347Ad582757)"
    echo "  ADMIN, TREASURY             default: the PRIVATE_KEY address"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run) DRY_RUN=true; shift ;;
        -c|--chains) CHAINS="$2"; shift 2 ;;
        --deposit) DEPOSIT="$2"; shift 2 ;;
        --stake) STAKE="$2"; shift 2 ;;
        -h|--help) print_usage; exit 0 ;;
        -*) echo -e "${RED}Error: Unknown option $1${NC}"; print_usage; exit 1 ;;
        *) STEPS+=("$1"); shift ;;
    esac
done

if [ ${#STEPS[@]} -eq 0 ]; then
    echo -e "${RED}Error: Missing step${NC}\n"
    print_usage
    exit 1
fi

die() { echo -e "${RED}Error: $*${NC}" >&2; exit 1; }
ok() { echo -e "  ${GREEN}✓${NC} $*"; }
info() { echo -e "  $*"; }
warn() { echo -e "  ${YELLOW}$*${NC}"; }
step() { echo ""; echo -e "${BLUE}━━ $* ━━${NC}"; }

lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }
same() { [ "$(lower "$1")" = "$(lower "$2")" ]; }

# Decimal strings, so values past 2^63 wei still compare.
at_least() {
    local a=$1 b=$2
    [ ${#a} -gt ${#b} ] && return 0
    [ ${#a} -lt ${#b} ] && return 1
    [[ "$a" > "$b" || "$a" == "$b" ]]
}

# One line of the env file; it is not valid shell, so never source it.
env_get() {
    local value
    value=$(grep -m1 "^$1=" "$ENV_FILE" 2>/dev/null | cut -d= -f2-) || true
    value=${value%$'\r'}
    value="${value%"${value##*[![:space:]]}"}"
    value=${value#\"}; value=${value%\"}
    value=${value#\'}; value=${value%\'}
    printf '%s' "$value"
}

chain_id() {
    case $1 in
        bsc-testnet) echo 97 ;;
        polygon-amoy) echo 80002 ;;
        *) die "Unsupported chain '$1': use bsc-testnet or polygon-amoy" ;;
    esac
}

rpc_var() {
    case $1 in
        bsc-testnet) echo BSC_TESTNET_RPC_URL ;;
        polygon-amoy) echo POLYGON_AMOY_RPC_URL ;;
    esac
}

rpc_fallback() {
    case $1 in
        bsc-testnet) echo BSC_CHAPEL ;;
        polygon-amoy) echo POLYGON_AMOY ;;
    esac
}

default_deposit() {
    case $1 in
        bsc-testnet) echo 0.3 ;;
        polygon-amoy) echo 1 ;;
    esac
}

default_stake() {
    case $1 in
        bsc-testnet) echo 0.05 ;;
        polygon-amoy) echo 0.1 ;;
    esac
}

rpc_url() { local var; var=$(rpc_var "$1"); printf '%s' "${!var}"; }

# Reads `[<chainId>.address] KEY` from the active config.
config_get() {
    awk -v section="[$1.address]" -v key="$2" '
        /^\[/ { found = ($0 == section); next }
        found && $1 == key { gsub(/"/, "", $3); print $3; exit }
    ' "$EVM_DIR/$CONFIG"
}

# The RPC rides in ETH_RPC_URL so it never shows up in argv or output.
rpc_call() {
    local chain=$1; shift
    ETH_RPC_URL="$(rpc_url "$chain")" cast call "$@" 2>/dev/null | awk '{print $1}' || true
}

has_code() {
    [ -n "$2" ] && [ "$(lower "$2")" != "0x0000000000000000000000000000000000000000" ] || return 1
    local code
    code=$(ETH_RPC_URL="$(rpc_url "$1")" cast code "$2" 2>/dev/null) || return 1
    [ -n "$code" ] && [ "$code" != "0x" ]
}

implementation_of() {
    local word
    word=$(ETH_RPC_URL="$(rpc_url "$1")" cast storage "$2" "$IMPLEMENTATION_SLOT" 2>/dev/null) || return 0
    cast parse-bytes32-address "$word"
}

# Runs one forge script on the chains given: straight onto the anvil forks in a dry run,
# through deploy.sh (confirmation prompt, broadcast, verification) otherwise.
run_script() {
    local script=$1; shift
    [ $# -gt 0 ] || return 0
    if $DRY_RUN; then
        local chain rpc
        for chain in "$@"; do
            rpc=$(rpc_url "$chain")
            [[ "$rpc" == http://127.0.0.1:* ]] || die "Dry run refuses a non-local RPC for $chain"
            info "${YELLOW}forge script $script on the $chain fork${NC}"
            (cd "$EVM_DIR" && forge script "script/$script.s.sol" --sig "run()" --rpc-url "$rpc" -g 150 \
                --broadcast --sender "$ADMIN") || die "$script failed on the $chain fork"
        done
    else
        local list
        list=$(IFS=,; echo "$*")
        (cd "$EVM_DIR" && bash script/deploy.sh --mode full -c "$CONFIG" "$script" "$list") ||
            warn "deploy.sh reported a failure (verification included); checking on-chain state"
    fi
}

# ── Environment ─────────────────────────────────────────────────────

IFS=',' read -ra CHAIN_ARRAY <<< "$CHAINS"
for chain in "${CHAIN_ARRAY[@]}"; do chain_id "$chain" > /dev/null; done

[ -f "$ENV_FILE" ] || die "Env file not found: $ENV_FILE"

PRIVATE_KEY="${PRIVATE_KEY:-$(env_get PRIVATE_KEY)}"
[ -n "$PRIVATE_KEY" ] || die "PRIVATE_KEY is unset and missing from $ENV_FILE"
[[ "$PRIVATE_KEY" == 0x* ]] || PRIVATE_KEY="0x$PRIVATE_KEY"
export PRIVATE_KEY
export ADMIN="${ADMIN:-$(cast wallet address --private-key "$PRIVATE_KEY")}"
export TREASURY="${TREASURY:-$ADMIN}"
export VERSION="${VERSION:-entrypoint-v09}"
export GOVERNANCE_RELAYER="${GOVERNANCE_RELAYER:-0xc8809DD0b00370be097382d741A43347Ad582757}"
# BaseScript reads both at construction; no step here uses them.
export CONSENSUS_STATE="${CONSENSUS_STATE:-0x}"
export SP1_VERIFICATION_KEY="${SP1_VERIFICATION_KEY:-0x0000000000000000000000000000000000000000000000000000000000000000}"
ETHEREUM_ETHERSCAN_API_KEY="${ETHEREUM_ETHERSCAN_API_KEY:-$(env_get ETHERSCAN_API)}"
export ETHEREUM_ETHERSCAN_API_KEY

for chain in "${CHAIN_ARRAY[@]}"; do
    var=$(rpc_var "$chain")
    [ -n "${!var}" ] || export "$var=$(env_get "$(rpc_fallback "$chain")")"
    [ -n "${!var}" ] || die "$var is unset and $(rpc_fallback "$chain") is missing from $ENV_FILE"
done

# The forge config loader resolves every ${VAR} in the TOML; chains outside this rollout get a stub.
for var in $(grep -o '\${[A-Za-z0-9_]*}' "$EVM_DIR/$CONFIG" | tr -d '${}' | sort -u); do
    [ -n "${!var}" ] || export "$var=http://127.0.0.1:1"
done

echo -e "${BLUE}EntryPoint v0.9 rollout${NC}"
echo -e "  Chains:    ${YELLOW}${CHAINS}${NC}"
echo -e "  Steps:     ${YELLOW}${STEPS[*]}${NC}"
echo -e "  Deployer:  ${YELLOW}${ADMIN}${NC}"
echo -e "  Treasury:  ${YELLOW}${TREASURY}${NC}"
echo -e "  Relayer:   ${YELLOW}${GOVERNANCE_RELAYER}${NC}"
echo -e "  VERSION:   ${YELLOW}${VERSION}${NC}"
echo -e "  Dry run:   ${YELLOW}${DRY_RUN}${NC}"

if $DRY_RUN; then
    TMP_DIR=$(mktemp -d)
    ANVIL_PIDS=()
    # forge only writes inside the project, so the copy lives next to the original.
    DRY_CONFIG="config.testnet.dryrun.$$.toml"
    cp "$EVM_DIR/$CONFIG" "$EVM_DIR/$DRY_CONFIG"
    CONFIG="$DRY_CONFIG"
    export FOUNDRY_BROADCAST="$TMP_DIR/broadcast"
    cleanup() {
        for pid in "${ANVIL_PIDS[@]}"; do kill "$pid" 2>/dev/null || true; done
        rm -f "$EVM_DIR/$DRY_CONFIG"
        rm -rf "$TMP_DIR"
    }
    trap cleanup EXIT

    step "Compiling before the forks start"
    (cd "$EVM_DIR" && forge build) || die "forge build failed"

    port=${ANVIL_PORT_BASE:-18545}
    for chain in "${CHAIN_ARRAY[@]}"; do
        var=$(rpc_var "$chain")
        anvil --fork-url "${!var}" --port "$port" --silent > /dev/null 2>&1 &
        ANVIL_PIDS+=($!)
        export "$var=http://127.0.0.1:$port"
        for _ in $(seq 1 60); do
            cast chain-id --rpc-url "http://127.0.0.1:$port" > /dev/null 2>&1 && break
            sleep 1
        done
        [ "$(cast chain-id --rpc-url "http://127.0.0.1:$port" 2>/dev/null)" = "$(chain_id "$chain")" ] ||
            die "The $chain fork did not come up on port $port"
        info "$chain forked on 127.0.0.1:$port"
        port=$((port + 1))
    done
fi
export CONFIG

# ── Steps ───────────────────────────────────────────────────────────

step_solver_account() {
    step "1. SolverAccount"
    local chain id account pending=()
    for chain in "${CHAIN_ARRAY[@]}"; do
        id=$(chain_id "$chain")
        account=$(config_get "$id" SOLVER_ACCOUNT)
        if has_code "$chain" "$account" && same "$(rpc_call "$chain" "$account" "entryPoint()(address)")" "$ENTRYPOINT_V09"; then
            ok "$chain: already on v0.9 at $account"
        else
            pending+=("$chain")
        fi
    done
    run_script DeploySolverAccount "${pending[@]}"

    local first=""
    for chain in "${CHAIN_ARRAY[@]}"; do
        account=$(config_get "$(chain_id "$chain")" SOLVER_ACCOUNT)
        same "$(rpc_call "$chain" "$account" "entryPoint()(address)")" "$ENTRYPOINT_V09" ||
            die "$chain: SolverAccount $account does not report EntryPoint v0.9"
        ok "$chain: SolverAccount $account, entryPoint() v0.9"
        [ -z "$first" ] && first=$account
        same "$first" "$account" || die "SolverAccount differs across chains: $first vs $account"
    done
    [ ${#CHAIN_ARRAY[@]} -gt 1 ] && ok "Same SolverAccount on every chain"
    return 0
}

step_gateway_impl() {
    step "2. IntentGatewayV2 implementation"
    local chain id gateway version impl pending=()
    for chain in "${CHAIN_ARRAY[@]}"; do
        id=$(chain_id "$chain")
        gateway=$(config_get "$id" INTENT_GATEWAY_V2)
        version=$(rpc_call "$chain" "$gateway" "version()(uint64)")
        [ "$version" = 3 ] || die "$chain: gateway $gateway is at version '$version'; the upgrade calldata assumes 3"
        impl=$(config_get "$id" INTENT_GATEWAY_V2_IMPL)
        if has_code "$chain" "$impl" && same "$(rpc_call "$chain" "$impl" "SELECT_SOLVER_TYPEHASH()(bytes32)")" "$SELECT_SOLVER_TYPEHASH"; then
            ok "$chain: implementation already deployed at $impl"
        else
            pending+=("$chain")
        fi
    done
    run_script DeployIntentGatewayImpl "${pending[@]}"

    for chain in "${CHAIN_ARRAY[@]}"; do
        impl=$(config_get "$(chain_id "$chain")" INTENT_GATEWAY_V2_IMPL)
        same "$(rpc_call "$chain" "$impl" "SELECT_SOLVER_TYPEHASH()(bytes32)")" "$SELECT_SOLVER_TYPEHASH" ||
            die "$chain: implementation $impl has the wrong SELECT_SOLVER_TYPEHASH"
        ok "$chain: implementation $impl"
        info "execute_on_gateway data: $(cast calldata "upgradeToAndCall(address,bytes)" "$impl" 0x)"
    done
}

paymaster_ready() {
    local paymaster
    paymaster=$(config_get "$(chain_id "$1")" SIMPLEX_PAYMASTER)
    has_code "$1" "$paymaster" &&
        [ "$(rpc_call "$1" "$paymaster" "version()(uint64)")" = 3 ] &&
        same "$(rpc_call "$1" "$paymaster" "entryPoint()(address)")" "$ENTRYPOINT_V09"
}

step_paymaster() {
    step "3. Fixed-price feeds and SimplexPaymaster"
    local chain id feeds=() paymasters=()
    for chain in "${CHAIN_ARRAY[@]}"; do
        id=$(chain_id "$chain")
        if paymaster_ready "$chain"; then
            ok "$chain: paymaster already deployed at $(config_get "$id" SIMPLEX_PAYMASTER)"
            continue
        fi
        paymasters+=("$chain")
        if has_code "$chain" "$(config_get "$id" NATIVE_ORACLE)" && has_code "$chain" "$(config_get "$id" USDC_ORACLE)"; then
            ok "$chain: feeds already deployed"
        else
            feeds+=("$chain")
        fi
    done
    run_script testnet/DeployFixedPriceFeed "${feeds[@]}"
    run_script DeploySimplexPaymaster "${paymasters[@]}"

    local paymaster host
    for chain in "${CHAIN_ARRAY[@]}"; do
        id=$(chain_id "$chain")
        paymaster=$(config_get "$id" SIMPLEX_PAYMASTER)
        paymaster_ready "$chain" || die "$chain: no v0.9 SimplexPaymaster at '$paymaster'"
        host=$(config_get "$id" HOST)
        same "$(rpc_call "$chain" "$paymaster" "host()(address)")" "$host" || die "$chain: paymaster host is not $host"
        same "$(rpc_call "$chain" "$paymaster" "relayer()(address)")" "$GOVERNANCE_RELAYER" ||
            die "$chain: paymaster relayer is not $GOVERNANCE_RELAYER"
        same "$(rpc_call "$chain" "$paymaster" "treasury()(address)")" "$TREASURY" ||
            die "$chain: paymaster treasury is not $TREASURY"
        ok "$chain: SimplexPaymaster $paymaster (implementation $(implementation_of "$chain" "$paymaster"))"
    done
}

# deposit, stake and unstake delay, one per line
deposit_info() {
    ETH_RPC_URL="$(rpc_url "$1")" cast call "$ENTRYPOINT_V09" \
        "getDepositInfo(address)(uint256,bool,uint112,uint32,uint48)" "$2" 2>/dev/null |
        awk 'NR == 1 || NR == 3 || NR == 4 {print $1}' || true
}

step_fund() {
    step "6. Paymaster deposit and stake"
    local chain paymaster deposit stake funds have_deposit have_stake have_delay
    for chain in "${CHAIN_ARRAY[@]}"; do
        paymaster=$(config_get "$(chain_id "$chain")" SIMPLEX_PAYMASTER)
        paymaster_ready "$chain" || die "$chain: deploy the paymaster first"
        deposit=$(cast to-wei "${DEPOSIT:-$(default_deposit "$chain")}")
        stake=$(cast to-wei "${STAKE:-$(default_stake "$chain")}")

        funds=$(deposit_info "$chain" "$paymaster")
        have_deposit=$(sed -n 1p <<< "$funds"); have_stake=$(sed -n 2p <<< "$funds"); have_delay=$(sed -n 3p <<< "$funds")
        if at_least "$have_deposit" "$deposit" && at_least "$have_stake" "$stake" && at_least "$have_delay" "$UNSTAKE_DELAY"; then
            ok "$chain: already funded"
        else
            export PAYMASTER_DEPOSIT=$deposit PAYMASTER_STAKE=$stake
            run_script testnet/FundSimplexPaymaster "$chain"
        fi

        funds=$(deposit_info "$chain" "$paymaster")
        have_deposit=$(sed -n 1p <<< "$funds"); have_stake=$(sed -n 2p <<< "$funds"); have_delay=$(sed -n 3p <<< "$funds")
        at_least "$have_deposit" "$deposit" || die "$chain: deposit $have_deposit is below $deposit"
        at_least "$have_stake" "$stake" || die "$chain: stake $have_stake is below $stake"
        at_least "$have_delay" "$UNSTAKE_DELAY" || die "$chain: unstake delay $have_delay is below $UNSTAKE_DELAY"
        ok "$chain: deposit $(cast from-wei "$have_deposit"), stake $(cast from-wei "$have_stake"), unstake delay ${have_delay}s"
    done
}

step_status() {
    step "Status"
    local chain id account gateway impl paymaster funds
    for chain in "${CHAIN_ARRAY[@]}"; do
        id=$(chain_id "$chain")
        echo -e "  ${YELLOW}$chain ($id)${NC}"
        account=$(config_get "$id" SOLVER_ACCOUNT)
        info "SolverAccount       $account entryPoint() $(rpc_call "$chain" "$account" "entryPoint()(address)")"
        gateway=$(config_get "$id" INTENT_GATEWAY_V2)
        impl=$(implementation_of "$chain" "$gateway")
        info "Gateway             $gateway version() $(rpc_call "$chain" "$gateway" "version()(uint64)")"
        info "  implementation    $impl (config: $(config_get "$id" INTENT_GATEWAY_V2_IMPL))"
        info "  typehash ok       $(same "$(rpc_call "$chain" "$gateway" "SELECT_SOLVER_TYPEHASH()(bytes32)")" "$SELECT_SOLVER_TYPEHASH" && echo yes || echo no)"
        paymaster=$(config_get "$id" SIMPLEX_PAYMASTER)
        if has_code "$chain" "$paymaster"; then
            info "SimplexPaymaster    $paymaster version() $(rpc_call "$chain" "$paymaster" "version()(uint64)")"
            info "  implementation    $(implementation_of "$chain" "$paymaster")"
            info "  relayer           $(rpc_call "$chain" "$paymaster" "relayer()(address)")"
            info "  treasury          $(rpc_call "$chain" "$paymaster" "treasury()(address)")"
            info "  bundlers          $(ETH_RPC_URL="$(rpc_url "$chain")" cast call "$paymaster" "getBundlers()(address[])" 2>/dev/null)"
            funds=$(deposit_info "$chain" "$paymaster")
            info "  deposit / stake   $(sed -n 1p <<< "$funds") / $(sed -n 2p <<< "$funds") wei, delay $(sed -n 3p <<< "$funds")s"
        else
            info "SimplexPaymaster    not deployed"
        fi
    done
}

for name in "${STEPS[@]}"; do
    case $name in
        solver-account) step_solver_account ;;
        gateway-impl) step_gateway_impl ;;
        paymaster) step_paymaster ;;
        fund) step_fund ;;
        status) step_status ;;
        *) die "Unknown step '$name'" ;;
    esac
done

echo ""
echo -e "${GREEN}✓ Done${NC}"
