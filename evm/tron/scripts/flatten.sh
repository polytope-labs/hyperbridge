#!/bin/bash

# Contract flattening script for TRON deployment
# Flattens Solidity contracts individually for verification on TronScan using TronBox
# Usage: ./scripts/flatten.sh [contract_name|all]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Output directory for flattened contracts
FLATTEN_DIR="flattened"

# Contract source paths (relative to evm directory)
declare -A CONTRACTS=(
    ["TronHost"]="src/hosts/Tron.sol"
    ["HandlerV1"]="src/core/HandlerV1.sol"
    ["HostManager"]="src/core/HostManager.sol"
    ["BeefyV1FiatShamir"]="src/consensus/BeefyV1FiatShamir.sol"
    ["ConsensusRouter"]="src/consensus/ConsensusRouter.sol"
    ["CallDispatcher"]="src/utils/CallDispatcher.sol"
    ["IntentGatewayV2"]="src/apps/IntentGatewayV2.sol"
)

# Function to print usage
print_usage() {
    echo -e "${BLUE}TRON Contract Flattening Script${NC}"
    echo ""
    echo "Usage: $0 [contract_name|all]"
    echo ""
    echo "Arguments:"
    echo "  contract_name    Name of contract to flatten (see list below)"
    echo "  all             Flatten all ISMP contracts (default)"
    echo ""
    echo "Examples:"
    echo "  # Flatten a specific contract"
    echo "  $0 TronHost"
    echo ""
    echo "  # Flatten all contracts"
    echo "  $0"
    echo "  $0 all"
    echo ""
    echo "Output:"
    echo "  Flattened contracts will be saved to: ${FLATTEN_DIR}/<ContractName>.sol"
    echo ""
    echo "Available Contracts:"
    for name in "${!CONTRACTS[@]}"; do
        echo "  - ${name}"
    done | sort
}

# Check if forge is installed (we'll use forge flatten instead of tronbox)
if ! command -v forge &> /dev/null; then
    echo -e "${RED}Error: forge not found. Please install Foundry:${NC}"
    echo "  curl -L https://foundry.paradigm.xyz | bash"
    echo "  foundryup"
    exit 1
fi

# Create flattened directory if it doesn't exist
mkdir -p "$FLATTEN_DIR"

# Function to flatten a single contract
flatten_contract() {
    local contract_name=$1
    local contract_path=$2

    echo -e "${YELLOW}Flattening ${contract_name}...${NC}"

    local output_file="${FLATTEN_DIR}/${contract_name}.sol"

    # Change to parent directory (evm) to access source files
    cd ..

    if forge flatten "$contract_path" -o "tron/${output_file}" 2>/dev/null; then
        cd tron

        # Check if file has content
        if [ -s "$output_file" ]; then
            echo -e "${GREEN}✓ ${contract_name} flattened successfully${NC}"
            echo -e "  Output: ${BLUE}${output_file}${NC}"

            # Get file size
            local file_size=$(du -h "$output_file" | cut -f1)
            echo -e "  Size: ${file_size}"
            echo ""
            return 0
        else
            cd tron
            echo -e "${RED}✗ ${contract_name} produced an empty file${NC}"
            rm -f "$output_file"
            echo ""
            return 1
        fi
    else
        cd tron
        echo -e "${RED}✗ Failed to flatten ${contract_name}${NC}"
        echo ""
        return 1
    fi
}

# Show help
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    print_usage
    exit 0
fi

# Print header
echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   TRON Contract Flattening Script      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

FAILED_CONTRACTS=()
SUCCESSFUL_CONTRACTS=()

# Determine which contracts to flatten
if [ $# -eq 0 ] || [ "$1" = "all" ]; then
    # Flatten all contracts
    echo -e "${GREEN}Flattening all ISMP contracts...${NC}"
    echo ""

    for contract_name in "${!CONTRACTS[@]}"; do
        contract_path="${CONTRACTS[$contract_name]}"
        if flatten_contract "$contract_name" "$contract_path"; then
            SUCCESSFUL_CONTRACTS+=("$contract_name")
        else
            FAILED_CONTRACTS+=("$contract_name")
        fi
    done
else
    # Flatten a specific contract
    CONTRACT_NAME=$1

    # Check if contract exists in our map
    if [ -z "${CONTRACTS[$CONTRACT_NAME]}" ]; then
        echo -e "${RED}Error: Unknown contract '${CONTRACT_NAME}'${NC}"
        echo ""
        echo -e "${YELLOW}Available contracts:${NC}"
        for name in "${!CONTRACTS[@]}"; do
            echo "  - $name"
        done | sort
        echo ""
        exit 1
    fi

    contract_path="${CONTRACTS[$CONTRACT_NAME]}"
    if flatten_contract "$CONTRACT_NAME" "$contract_path"; then
        SUCCESSFUL_CONTRACTS+=("$CONTRACT_NAME")
    else
        FAILED_CONTRACTS+=("$CONTRACT_NAME")
    fi
fi

# Print summary
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo -e "${YELLOW}Flattening Summary${NC}"
echo -e "${BLUE}════════════════════════════════════════${NC}"
echo ""

if [ ${#SUCCESSFUL_CONTRACTS[@]} -gt 0 ]; then
    echo -e "${GREEN}Successfully flattened (${#SUCCESSFUL_CONTRACTS[@]}):${NC}"
    for contract in "${SUCCESSFUL_CONTRACTS[@]}"; do
        echo -e "  ✓ ${contract}"
    done | sort
    echo ""
fi

if [ ${#FAILED_CONTRACTS[@]} -gt 0 ]; then
    echo -e "${RED}Failed (${#FAILED_CONTRACTS[@]}):${NC}"
    for contract in "${FAILED_CONTRACTS[@]}"; do
        echo -e "  ✗ ${contract}"
    done | sort
    echo ""
fi

# Final status
if [ ${#FAILED_CONTRACTS[@]} -eq 0 ]; then
    echo -e "${GREEN}✓ All contracts flattened successfully!${NC}"
    echo ""
    echo -e "${YELLOW}Output directory:${NC} ${BLUE}${FLATTEN_DIR}/${NC}"
    echo ""
    echo -e "${YELLOW}Next steps for contract verification on TronScan:${NC}"
    echo ""
    echo "  1. Navigate to your deployed contract:"
    echo "     • Nile testnet: https://nile.tronscan.org/#/contract/<address>/code"
    echo "     • Shasta testnet: https://shasta.tronscan.org/#/contract/<address>/code"
    echo "     • Mainnet: https://tronscan.org/#/contract/<address>/code"
    echo ""
    echo "  2. Click the 'Verify Contract' button"
    echo ""
    echo "  3. Fill in the verification form:"
    echo "     • Contract Name: e.g., TronHost, HandlerV1, etc."
    echo "     • Compiler Version: 0.8.24"
    echo "     • Optimization: Enabled"
    echo "     • Optimization Runs: 200"
    echo "     • EVM Version: paris"
    echo ""
    echo "  4. Upload the corresponding flattened file from ${FLATTEN_DIR}/"
    echo "     Example: For TronHost, upload TronHost.sol"
    echo ""
    echo "  5. Submit for verification"
    echo ""
    exit 0
else
    echo -e "${RED}✗ Some contracts failed to flatten${NC}"
    exit 1
fi
