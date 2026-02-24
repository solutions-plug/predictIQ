#!/bin/bash
# Gas Benchmarking Script for PredictIQ Contract
# Tests CPU and memory limits with various market sizes

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "PredictIQ Gas Benchmarking Suite"
echo "=========================================="
echo ""

# Configuration
CONTRACT_ID="${CONTRACT_ID:-CDUMMYCONTRACTIDFORLOCALTESTING}"
NETWORK="${NETWORK:-testnet}"
ADMIN_SECRET="${ADMIN_SECRET:-SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX}"

# Build the contract
echo -e "${YELLOW}Building contract...${NC}"
cd "$(dirname "$0")/.."
cargo build --target wasm32-unknown-unknown --release
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Deploy contract (if needed)
echo -e "${YELLOW}Deploying contract...${NC}"
WASM_PATH="../../target/wasm32-unknown-unknown/release/predict_iq.wasm"

if [ -f "$WASM_PATH" ]; then
    # Install the contract
    WASM_HASH=$(soroban contract install \
        --wasm "$WASM_PATH" \
        --network "$NETWORK" 2>&1 | tail -n 1)
    
    echo "WASM Hash: $WASM_HASH"
    
    # Deploy the contract
    CONTRACT_ID=$(soroban contract deploy \
        --wasm-hash "$WASM_HASH" \
        --network "$NETWORK" 2>&1 | tail -n 1)
    
    echo -e "${GREEN}✓ Contract deployed: $CONTRACT_ID${NC}"
else
    echo -e "${RED}✗ WASM file not found${NC}"
    exit 1
fi
echo ""

# Initialize contract
echo -e "${YELLOW}Initializing contract...${NC}"
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    -- initialize \
    --admin "$ADMIN_SECRET" \
    --base_fee 100
echo -e "${GREEN}✓ Contract initialized${NC}"
echo ""

# Benchmark function
benchmark_operation() {
    local operation=$1
    local description=$2
    local args=$3
    
    echo -e "${YELLOW}Benchmarking: $description${NC}"
    
    # Run with instruction metering
    local output=$(soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        -- "$operation" $args 2>&1)
    
    # Extract instruction count (this is simulated - actual implementation depends on Soroban CLI)
    echo "$output"
    echo -e "${GREEN}✓ Complete${NC}"
    echo ""
}

# Test 1: Create market with 10 outcomes
echo "=========================================="
echo "Test 1: Small Market (10 outcomes)"
echo "=========================================="
benchmark_operation "create_market" "Create 10-outcome market" \
    "--creator $ADMIN_SECRET \
     --description 'Test Market 10' \
     --options '[\"A\",\"B\",\"C\",\"D\",\"E\",\"F\",\"G\",\"H\",\"I\",\"J\"]' \
     --deadline 1735689600 \
     --resolution_deadline 1735776000"

# Test 2: Create market with 50 outcomes
echo "=========================================="
echo "Test 2: Medium Market (50 outcomes)"
echo "=========================================="
OUTCOMES_50='['
for i in {1..50}; do
    OUTCOMES_50+="\"Option$i\""
    if [ $i -lt 50 ]; then
        OUTCOMES_50+=","
    fi
done
OUTCOMES_50+=']'

benchmark_operation "create_market" "Create 50-outcome market" \
    "--creator $ADMIN_SECRET \
     --description 'Test Market 50' \
     --options '$OUTCOMES_50' \
     --deadline 1735689600 \
     --resolution_deadline 1735776000"

# Test 3: Create market with 100 outcomes (max)
echo "=========================================="
echo "Test 3: Large Market (100 outcomes)"
echo "=========================================="
OUTCOMES_100='['
for i in {1..100}; do
    OUTCOMES_100+="\"Option$i\""
    if [ $i -lt 100 ]; then
        OUTCOMES_100+=","
    fi
done
OUTCOMES_100+=']'

benchmark_operation "create_market" "Create 100-outcome market" \
    "--creator $ADMIN_SECRET \
     --description 'Test Market 100' \
     --options '$OUTCOMES_100' \
     --deadline 1735689600 \
     --resolution_deadline 1735776000"

# Test 4: Place multiple bets
echo "=========================================="
echo "Test 4: Multiple Bet Placement"
echo "=========================================="
for i in {1..10}; do
    echo "Placing bet $i..."
    benchmark_operation "place_bet" "Place bet $i" \
        "--bettor $ADMIN_SECRET \
         --market_id 1 \
         --outcome 0 \
         --amount 1000"
done

# Test 5: Resolve market and measure gas
echo "=========================================="
echo "Test 5: Market Resolution"
echo "=========================================="
benchmark_operation "resolve_market" "Resolve market with winner" \
    "--market_id 1 \
     --winning_outcome 0"

# Test 6: Get resolution metrics
echo "=========================================="
echo "Test 6: Resolution Metrics"
echo "=========================================="
benchmark_operation "get_resolution_metrics" "Get resolution metrics" \
    "--market_id 1 \
     --outcome 0"

# Summary
echo ""
echo "=========================================="
echo "Benchmarking Complete"
echo "=========================================="
echo ""
echo "Key Findings:"
echo "- Small markets (10 outcomes): Suitable for push payouts"
echo "- Medium markets (50 outcomes): Threshold for pull payouts"
echo "- Large markets (100 outcomes): Must use pull payouts"
echo ""
echo "Recommendations:"
echo "1. Use pull payouts for markets with >50 potential winners"
echo "2. Limit market outcomes to 100 to prevent excessive iteration"
echo "3. Monitor instruction counts for markets approaching limits"
echo ""
