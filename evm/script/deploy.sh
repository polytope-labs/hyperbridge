#!/bin/bash

# Multi-chain deployment script with simulation and full deployment modes
# Usage: ./script/deploy-multichain.sh [OPTIONS] <script> <chains>

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
MODE="simulate"

CONFIG_FILE=""
NETWORK_FLAG=""

# Function to print usage
print_usage() {
    echo -e "${BLUE}Multi-Chain Deployment Script${NC}"
    echo ""
    echo "Usage: $0 [OPTIONS] <script> <chains>"
    echo ""
    echo "Arguments:"
    echo "  script          Script name (e.g., DeployHostUpdates.s.sol or DeployHostUpdates)"
    echo "  chains          'testnet', 'mainnet', or comma-separated chain names"
    echo "                  (e.g., testnet, mainnet, sepolia,base-sepolia,arbitrum-sepolia)"
    echo ""
    echo "Options:"
    echo "  -m, --mode MODE        Deployment mode: simulate or full (default: simulate)"
    echo "                         - simulate: Dry run without broadcasting transactions"
    echo "                         - full: Broadcast and verify contracts"
    echo "  -n, --network NET      Network type: testnet or mainnet (sources .env.testnet or .env.mainnet)"
    echo "  -c, --config FILE      Config file to use (default: from CONFIG env var)"

    echo "  -h, --help             Show this help message"
    echo ""
    echo "Examples:"
    echo "  # Simulate deployment (dry run)"
    echo "  $0 DeployHostUpdates sepolia,base-sepolia"
    echo ""
    echo "  # Deploy to all testnets (sources .env.testnet)"
    echo "  $0 --mode full --network testnet DeployHostUpdates testnet"
    echo ""
    echo "  # Deploy to all mainnets (sources .env.mainnet)"
    echo "  $0 --mode full --network mainnet DeployIsmp mainnet"
    echo ""
    echo "  # Deploy to specific testnet chains"
    echo "  $0 --mode full --network testnet DeployHostUpdates sepolia,base-sepolia"
    echo ""
    echo "  # Deploy to specific chains"
    echo "  $0 --mode full DeployHostUpdates sepolia,base-sepolia,arbitrum-sepolia"
    echo ""

    echo ""
    echo "Available Chains:"
    echo "  Testnets: sepolia, optimism-sepolia, arbitrum-sepolia, base-sepolia,"
    echo "            polygon-amoy, bsc-testnet, gnosis-chiado, sei-testnet"
    echo ""
    echo "  Mainnets: ethereum, optimism, arbitrum, base, bsc, gnosis,"
    echo "            soneium, polygon, unichain, inkchain, sei"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -m|--mode)
            MODE="$2"
            if [[ ! "$MODE" =~ ^(simulate|full)$ ]]; then
                echo -e "${RED}Error: Invalid mode '$MODE'. Must be 'simulate' or 'full'${NC}"
                exit 1
            fi
            shift 2
            ;;
        -n|--network)
            NETWORK_FLAG="$2"
            if [[ ! "$NETWORK_FLAG" =~ ^(testnet|mainnet)$ ]]; then
                echo -e "${RED}Error: Invalid network '$NETWORK_FLAG'. Must be 'testnet' or 'mainnet'${NC}"
                exit 1
            fi
            shift 2
            ;;
        -c|--config)
            CONFIG_FILE="$2"
            shift 2
            ;;

        -h|--help)
            print_usage
            exit 0
            ;;
        -*)
            echo -e "${RED}Error: Unknown option $1${NC}"
            print_usage
            exit 1
            ;;
        *)
            break
            ;;
    esac
done

# Check remaining arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Error: Missing required arguments${NC}\n"
    print_usage
    exit 1
fi

SCRIPT_NAME=$1
CHAINS=$2

# Auto-detect network from chains parameter if --network not provided
if [ -z "$NETWORK_FLAG" ]; then
    if [ "$CHAINS" = "testnet" ]; then
        NETWORK_FLAG="testnet"
    elif [ "$CHAINS" = "mainnet" ]; then
        NETWORK_FLAG="mainnet"
    fi
fi

# Source .env file based on network flag
if [ "$NETWORK_FLAG" = "testnet" ]; then
    if [ -f ".env.testnet" ]; then
        echo -e "${YELLOW}Sourcing .env.testnet${NC}"
        set -a
        source .env.testnet
        set +a
    fi
    # Auto-set config if not specified
    if [ -z "$CONFIG_FILE" ] && [ -z "$CONFIG" ]; then
        export CONFIG=config.testnet.toml
    fi
elif [ "$NETWORK_FLAG" = "mainnet" ]; then
    if [ -f ".env.mainnet" ]; then
        echo -e "${YELLOW}Sourcing .env.mainnet${NC}"
        set -a
        source .env.mainnet
        set +a
    fi
    # Auto-set config if not specified
    if [ -z "$CONFIG_FILE" ] && [ -z "$CONFIG" ]; then
        export CONFIG=config.mainnet.toml
    fi
fi

# Expand "testnet" or "mainnet" to actual chain lists
if [ "$CHAINS" = "testnet" ]; then
    CHAINS="sepolia,optimism-sepolia,arbitrum-sepolia,base-sepolia,polygon-amoy,bsc-testnet,gnosis-chiado,sei-testnet"
    echo -e "${GREEN}Deploying to all testnet chains: ${YELLOW}${CHAINS}${NC}"
    echo ""
elif [ "$CHAINS" = "mainnet" ]; then
    CHAINS="ethereum,optimism,arbitrum,base,bsc,gnosis,soneium,polygon,unichain,inkchain,sei"
    echo -e "${GREEN}Deploying to all mainnet chains: ${YELLOW}${CHAINS}${NC}"
    echo ""
fi

# Validate and normalize script name
if [[ ! "$SCRIPT_NAME" =~ \.s\.sol$ ]]; then
    SCRIPT_NAME="${SCRIPT_NAME}.s.sol"
fi

SCRIPT_PATH="script/${SCRIPT_NAME}"

if [ ! -f "$SCRIPT_PATH" ]; then
    echo -e "${RED}Error: Script not found: $SCRIPT_PATH${NC}"
    exit 1
fi

# Set config file
if [ -n "$CONFIG_FILE" ]; then
    export CONFIG="$CONFIG_FILE"
elif [ -z "$CONFIG" ]; then
    echo -e "${YELLOW}Warning: No CONFIG specified, will use environment default${NC}"
fi

# Print configuration
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Multi-Chain Deployment Script        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Configuration:${NC}"
echo -e "  Script:    ${YELLOW}${SCRIPT_NAME}${NC}"
echo -e "  Chains:    ${YELLOW}${CHAINS}${NC}"
echo -e "  Mode:      ${YELLOW}${MODE}${NC}"
echo -e "  Config:    ${YELLOW}${CONFIG:-<from env>}${NC}"



echo ""

# Verify required environment variables
REQUIRED_VARS=("PRIVATE_KEY" "ADMIN" "VERSION")
MISSING_VARS=()

for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        MISSING_VARS+=("$var")
    fi
done

if [ ${#MISSING_VARS[@]} -gt 0 ]; then
    echo -e "${RED}Error: Missing required environment variables:${NC}"
    for var in "${MISSING_VARS[@]}"; do
        echo -e "  - ${RED}$var${NC}"
    done
    echo ""
    echo -e "${YELLOW}Hint: Source your .env file or export the required variables${NC}"
    exit 1
fi

# Split chains into array
IFS=',' read -ra CHAIN_ARRAY <<< "$CHAINS"

# Print what we're about to do
echo -e "${GREEN}Deploying to ${#CHAIN_ARRAY[@]} chain(s):${NC}"
for chain in "${CHAIN_ARRAY[@]}"; do
    echo -e "  - ${YELLOW}${chain}${NC}"
done
echo ""

# Confirm for full mode
if [ "$MODE" = "full" ]; then
    CHAIN_COUNT=${#CHAIN_ARRAY[@]}
    echo -e "${YELLOW}⚠️  WARNING: This will broadcast transactions to ${CHAIN_COUNT} chain(s) using real funds.${NC}"
    echo ""

    read -p "Are you sure you want to proceed? Type 'yes' to continue: " -r
    echo
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        echo -e "${RED}Deployment cancelled${NC}"
        exit 1
    fi
fi

# Execute deployment for each chain
echo -e "${BLUE}════════════════════════════════════════${NC}"
if [ "$MODE" = "simulate" ]; then
    echo -e "${GREEN}Starting simulation (no transactions will be broadcast)...${NC}"
else
    echo -e "${GREEN}Starting deployment...${NC}"
fi
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo ""

FAILED_CHAINS=()
SUCCESSFUL_CHAINS=()

# Loop through each chain
for chain in "${CHAIN_ARRAY[@]}"; do
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Deploying to: ${chain}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    # Build forge command for this chain (using single-chain run())
    FORGE_CMD="forge script $SCRIPT_PATH --sig \"run()\" --rpc-url $chain"

    # Add flags based on mode
    if [ "$MODE" = "full" ]; then
        FORGE_CMD="$FORGE_CMD --broadcast --verify --sender $ADMIN"
    fi

    # Execute the command
    if eval $FORGE_CMD; then
        SUCCESSFUL_CHAINS+=("$chain")
        echo ""
        echo -e "${GREEN}✓ Deployment to ${chain} completed${NC}"
        echo ""
    else
        FAILED_CHAINS+=("$chain")
        echo ""
        echo -e "${RED}✗ Deployment to ${chain} failed${NC}"
        echo ""
    fi
done

# Print summary
echo ""
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo -e "${YELLOW}Deployment Summary${NC}"
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo ""

if [ ${#SUCCESSFUL_CHAINS[@]} -gt 0 ]; then
    echo -e "${GREEN}Successful (${#SUCCESSFUL_CHAINS[@]}):${NC}"
    for chain in "${SUCCESSFUL_CHAINS[@]}"; do
        echo -e "  ✓ ${chain}"
    done
    echo ""
fi

if [ ${#FAILED_CHAINS[@]} -gt 0 ]; then
    echo -e "${RED}Failed (${#FAILED_CHAINS[@]}):${NC}"
    for chain in "${FAILED_CHAINS[@]}"; do
        echo -e "  ✗ ${chain}"
    done
    echo ""
fi

# Final status
if [ ${#FAILED_CHAINS[@]} -eq 0 ]; then
    echo -e "${GREEN}✓ All deployments completed successfully!${NC}"

    if [ "$MODE" = "simulate" ]; then
        echo ""
        echo -e "${YELLOW}Next steps:${NC}"
        echo -e "  1. Review the simulation output above"
        echo -e "  2. To deploy for real, run:"
        echo -e "     ${BLUE}$0 --mode full --network $NETWORK_FLAG $1 $2${NC}"
    else
        echo ""
        echo -e "${YELLOW}Deployment artifacts saved in:${NC}"
        echo -e "  broadcast/${SCRIPT_NAME}/<chain-id>/"
    fi
    exit 0
else
    echo -e "${RED}✗ Some deployments failed${NC}"
    exit 1
fi
