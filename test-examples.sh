#!/bin/bash

# Script to test all LTL examples
# Usage: ./test-ltl-examples.sh

set -e

EXAMPLES_DIR="examples"
FAILED_TESTS=()
PASSED_TESTS=()
TOTAL=0

echo "======================================"
echo "Testing LTL Examples"
echo "======================================"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Map file names to expected outcomes
get_expected_result() {
    case "$1" in
        ltl-always-eventually-fail.alt) echo fail ;;
        ltl-broadcast-fail.alt) echo fail ;;
        ltl-broadcast-pass.alt) echo pass ;;
        ltl-implication-fail.alt) echo fail ;;
        ltl-safety-simple.alt) echo pass ;;
        ltl-safety-violation.alt) echo fail ;;
        ltl-eventually-simple.alt) echo pass ;;
        ltl-always-eventually.alt) echo pass ;;
        ltl-boolean-workflow.alt) echo pass ;;
        ltl-mutual-exclusion.alt) echo fail ;;
        ltl-bounded-buffer.alt) echo pass ;;
        ltl-state-machine.alt) echo pass ;;
        ltl-resource-allocation.alt) echo pass ;;
        ltl-deadlock-freedom.alt) echo pass ;;
        ltl-implications.alt) echo pass ;;
        ltl-multiple-properties.alt) echo pass ;;
        *) echo pass ;;
    esac
}

# Test each LTL example file
for file in "$EXAMPLES_DIR"/*.alt; do
    if [ ! -f "$file" ]; then
        continue
    fi
    
    filename=$(basename "$file")
    expected=$(get_expected_result "$filename")
    
    TOTAL=$((TOTAL + 1))
    echo -n "Testing $filename ... "
    
    # Run the checker and capture output
    if cargo run --quiet --bin althread-cli -- check "$file" > /dev/null 2>&1; then
        actual="pass"
    else
        actual="fail"
    fi
    
    # Check if result matches expectation
    if [ "$actual" == "$expected" ]; then
        echo -e "${GREEN}✓ OK${NC} (expected: $expected, got: $actual)"
        PASSED_TESTS+=("$filename")
    else
        echo -e "${RED}✗ FAILED${NC} (expected: $expected, got: $actual)"
        FAILED_TESTS+=("$filename")
    fi
done

echo ""
echo "======================================"
echo "Test Results"
echo "======================================"
echo "Total tests: $TOTAL"
echo -e "${GREEN}Passed: ${#PASSED_TESTS[@]}${NC}"
echo -e "${RED}Failed: ${#FAILED_TESTS[@]}${NC}"

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo ""
    echo "Failed tests:"
    for test in "${FAILED_TESTS[@]}"; do
        echo "  - $test"
    done
    exit 1
else
    echo ""
    echo -e "${GREEN}All tests passed! ✓${NC}"
    exit 0
fi
