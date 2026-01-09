#!/usr/bin/env bash
# bootstrap.sh - Build/test/bench script for alembic-rs
#
# Commands:
#   test      - Run all tests
#   build     - Build release
#   check     - cargo check + clippy
#   bench     - Benchmark file reading
#   clean     - Clean build artifacts
#   help      - Show this help

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;91m'
GREEN='\033[0;92m'
YELLOW='\033[0;93m'
CYAN='\033[0;96m'
NC='\033[0m' # No Color

# Test files
declare -A TEST_FILES=(
    ["chess3"]="data/chess3.abc"
    ["chess4"]="data/chess4.abc"
    ["bmw"]="data/bmw.abc"
)

# ============================================================
# UTILITY FUNCTIONS
# ============================================================

fmt_time() {
    local ms=$1
    if (( ms < 1000 )); then
        echo "${ms}ms"
    elif (( ms < 60000 )); then
        echo "$(echo "scale=1; $ms/1000" | bc)s"
    else
        local mins=$((ms / 60000))
        local secs=$(((ms % 60000) / 1000))
        echo "${mins}m${secs}s"
    fi
}

print_header() {
    local text=$1
    local line="============================================================"
    echo ""
    echo -e "${CYAN}${line}"
    echo "$text"
    echo -e "${line}${NC}"
}

print_subheader() {
    echo ""
    echo -e "${YELLOW}[$1]${NC}"
}

# ============================================================
# HELP
# ============================================================

show_help() {
    cat << 'EOF'

 ALEMBIC-RS BUILD SCRIPT

 COMMANDS
   test      Run all tests (unit + integration)
   build     Build release binary
   check     Run cargo check + clippy
   bench     Benchmark file reading performance
   clean     Clean build artifacts

 OPTIONS
   -v, --verbose  Show detailed output

 EXAMPLES
   ./bootstrap.sh test           # Run all tests
   ./bootstrap.sh build          # Build release
   ./bootstrap.sh bench          # Benchmark reading
   ./bootstrap.sh check          # Check + clippy

EOF
}

# ============================================================
# TEST MODE
# ============================================================

cmd_test() {
    local verbose=$1
    print_header "ALEMBIC-RS TESTS"
    
    local start=$(date +%s%3N)
    
    print_subheader "Unit Tests"
    if [[ "$verbose" == "1" ]]; then
        cargo test --lib -- --nocapture
    else
        cargo test --lib
    fi
    local code1=$?
    
    print_subheader "Integration Tests"
    if [[ "$verbose" == "1" ]]; then
        cargo test --test read_files -- --nocapture
    else
        cargo test --test read_files
    fi
    local code2=$?
    
    local end=$(date +%s%3N)
    local elapsed=$((end - start))
    
    print_header "RESULTS"
    echo ""
    if [[ $code1 -eq 0 && $code2 -eq 0 ]]; then
        echo -e "  ${GREEN}All tests passed!${NC}"
    else
        echo -e "  ${RED}Some tests failed${NC}"
    fi
    echo -e "  ${CYAN}Time: $(fmt_time $elapsed)${NC}"
    echo ""
    
    [[ $code1 -eq 0 && $code2 -eq 0 ]] && return 0 || return 1
}

# ============================================================
# BUILD MODE
# ============================================================

cmd_build() {
    print_header "BUILD RELEASE"
    
    local start=$(date +%s%3N)
    cargo build --release
    local code=$?
    local end=$(date +%s%3N)
    local elapsed=$((end - start))
    
    echo ""
    if [[ $code -eq 0 ]]; then
        echo -e "  ${GREEN}Build successful!${NC}"
        echo -e "  ${CYAN}Time: $(fmt_time $elapsed)${NC}"
    else
        echo -e "  ${RED}Build failed${NC}"
    fi
    echo ""
    
    return $code
}

# ============================================================
# CHECK MODE
# ============================================================

cmd_check() {
    print_header "CHECK + CLIPPY"
    
    print_subheader "cargo check"
    cargo check
    local code1=$?
    
    print_subheader "cargo clippy"
    cargo clippy -- -D warnings
    local code2=$?
    
    echo ""
    if [[ $code1 -eq 0 && $code2 -eq 0 ]]; then
        echo -e "  ${GREEN}All checks passed!${NC}"
    else
        echo -e "  ${RED}Checks failed${NC}"
    fi
    echo ""
    
    [[ $code1 -eq 0 && $code2 -eq 0 ]] && return 0 || return 1
}

# ============================================================
# BENCH MODE
# ============================================================

cmd_bench() {
    print_header "BENCHMARK FILE READING"
    
    print_subheader "Building release"
    cargo build --release --quiet
    
    print_subheader "Reading test files"
    
    for name in "${!TEST_FILES[@]}"; do
        local path="${SCRIPT_DIR}/${TEST_FILES[$name]}"
        if [[ ! -f "$path" ]]; then
            echo -e "  $name... ${YELLOW}SKIP (file not found)${NC}"
            continue
        fi
        
        local size=$(du -h "$path" | cut -f1)
        echo -n "  $name ($size)... "
        
        local start=$(date +%s%3N)
        cargo test "test_open_$name" --release --quiet 2>/dev/null
        local code=$?
        local end=$(date +%s%3N)
        local elapsed=$((end - start))
        
        if [[ $code -eq 0 ]]; then
            echo -e "${GREEN}$(fmt_time $elapsed)${NC}"
        else
            echo -e "${RED}FAIL${NC}"
        fi
    done
    
    print_subheader "Full geometry scan (BMW)"
    local start=$(date +%s%3N)
    cargo test test_bmw_geometry --release -- --nocapture 2>&1 | grep -E "Total|vertices|faces" || true
    local end=$(date +%s%3N)
    local elapsed=$((end - start))
    echo -e "  ${CYAN}Scan time: $(fmt_time $elapsed)${NC}"
    echo ""
}

# ============================================================
# CLEAN MODE
# ============================================================

cmd_clean() {
    print_header "CLEAN"
    
    cargo clean
    
    if [[ -d "test/out" ]]; then
        rm -rf test/out
        echo -e "  ${YELLOW}Removed test/out${NC}"
    fi
    
    echo -e "  ${GREEN}Done!${NC}"
    echo ""
}

# ============================================================
# MAIN
# ============================================================

VERBOSE=0
COMMAND=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help|help)
            show_help
            exit 0
            ;;
        test|build|check|bench|clean)
            COMMAND=$1
            shift
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

if [[ -z "$COMMAND" ]]; then
    show_help
    exit 0
fi

case $COMMAND in
    test)  cmd_test $VERBOSE ;;
    build) cmd_build ;;
    check) cmd_check ;;
    bench) cmd_bench ;;
    clean) cmd_clean ;;
esac
