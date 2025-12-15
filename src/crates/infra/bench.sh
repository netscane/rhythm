#!/bin/bash

# MemtableContext Benchmark Runner Script
# Provides convenient commands for common benchmark scenarios

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

function print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

function print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

function print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

function print_error() {
    echo -e "${RED}✗ $1${NC}"
}

function show_usage() {
    cat << EOF
MemtableContext Benchmark Runner

Usage: ./bench.sh [command] [options]

Commands:
    all                 Run all benchmarks (default)
    quick               Run quick benchmarks (reduced sample size)
    sequential          Run only sequential insert benchmarks
    concurrent          Run only concurrent benchmarks
    rotation            Run only rotation & flush benchmarks
    reads               Run only read-related benchmarks
    writes              Run only write-related benchmarks
    mixed               Run only mixed read-write benchmarks
    baseline <name>     Save current results as baseline
    compare <name>      Compare with saved baseline
    clean               Clean benchmark results
    report              Open HTML report in browser
    flamegraph          Generate CPU flamegraph
    help                Show this help message

Options:
    --sample-size N     Set sample size (default: 100)
    --warm-up-time N    Set warm-up time in seconds (default: 3)

Examples:
    ./bench.sh all                          # Run all benchmarks
    ./bench.sh quick                        # Run quick benchmarks
    ./bench.sh baseline before-opt          # Save baseline
    ./bench.sh compare before-opt           # Compare with baseline
    ./bench.sh concurrent --sample-size 50  # Run concurrent with custom samples
    ./bench.sh flamegraph                   # Generate flamegraph

EOF
}

function run_all() {
    print_header "Running All Benchmarks"
    cargo bench --bench memtable_benchmark "$@"
    print_success "All benchmarks completed"
}

function run_quick() {
    print_header "Running Quick Benchmarks"
    print_warning "Using reduced sample size for faster results"
    cargo bench --bench memtable_benchmark -- --sample-size 10 --warm-up-time 1 "$@"
    print_success "Quick benchmarks completed"
}

function run_sequential() {
    print_header "Running Sequential Insert Benchmarks"
    cargo bench --bench memtable_benchmark -- sequential_inserts "$@"
    print_success "Sequential benchmarks completed"
}

function run_concurrent() {
    print_header "Running Concurrent Benchmarks"
    cargo bench --bench memtable_benchmark -- concurrent "$@"
    print_success "Concurrent benchmarks completed"
}

function run_rotation() {
    print_header "Running Rotation & Flush Benchmarks"
    cargo bench --bench memtable_benchmark -- rotation "$@"
    print_success "Rotation benchmarks completed"
}

function run_reads() {
    print_header "Running Read Benchmarks"
    cargo bench --bench memtable_benchmark -- "get_operations\|index_lookups\|concurrent_read" "$@"
    print_success "Read benchmarks completed"
}

function run_writes() {
    print_header "Running Write Benchmarks"
    cargo bench --bench memtable_benchmark -- "sequential_inserts\|concurrent_inserts\|delete\|update" "$@"
    print_success "Write benchmarks completed"
}

function run_mixed() {
    print_header "Running Mixed Read-Write Benchmarks"
    cargo bench --bench memtable_benchmark -- mixed_read_write "$@"
    print_success "Mixed benchmarks completed"
}

function save_baseline() {
    local baseline_name="$1"
    if [ -z "$baseline_name" ]; then
        print_error "Baseline name required"
        echo "Usage: ./bench.sh baseline <name>"
        exit 1
    fi
    
    print_header "Saving Baseline: $baseline_name"
    cargo bench --bench memtable_benchmark -- --save-baseline "$baseline_name"
    print_success "Baseline '$baseline_name' saved"
}

function compare_baseline() {
    local baseline_name="$1"
    if [ -z "$baseline_name" ]; then
        print_error "Baseline name required"
        echo "Usage: ./bench.sh compare <name>"
        exit 1
    fi
    
    print_header "Comparing with Baseline: $baseline_name"
    cargo bench --bench memtable_benchmark -- --baseline "$baseline_name"
    print_success "Comparison with '$baseline_name' completed"
}

function clean_results() {
    print_header "Cleaning Benchmark Results"
    rm -rf ../../target/criterion
    print_success "Benchmark results cleaned"
}

function open_report() {
    local report_path="../../target/criterion/report/index.html"
    
    if [ ! -f "$report_path" ]; then
        print_error "No benchmark report found. Run benchmarks first."
        exit 1
    fi
    
    print_header "Opening Benchmark Report"
    
    # Detect OS and open browser
    if command -v xdg-open &> /dev/null; then
        xdg-open "$report_path"
    elif command -v open &> /dev/null; then
        open "$report_path"
    elif command -v start &> /dev/null; then
        start "$report_path"
    else
        print_warning "Could not detect browser command"
        echo "Please open: $report_path"
    fi
    
    print_success "Report opened"
}

function generate_flamegraph() {
    print_header "Generating CPU Flamegraph"
    
    if ! command -v flamegraph &> /dev/null; then
        print_warning "flamegraph not installed, installing..."
        cargo install flamegraph
    fi
    
    print_warning "This may require sudo for perf access"
    cargo flamegraph --bench memtable_benchmark
    
    if [ -f "flamegraph.svg" ]; then
        print_success "Flamegraph generated: flamegraph.svg"
        
        # Try to open it
        if command -v xdg-open &> /dev/null; then
            xdg-open flamegraph.svg
        elif command -v open &> /dev/null; then
            open flamegraph.svg
        fi
    else
        print_error "Failed to generate flamegraph"
        exit 1
    fi
}

# Parse command
COMMAND="${1:-all}"
shift || true

# Execute command
case "$COMMAND" in
    all)
        run_all "$@"
        ;;
    quick)
        run_quick "$@"
        ;;
    sequential)
        run_sequential "$@"
        ;;
    concurrent)
        run_concurrent "$@"
        ;;
    rotation)
        run_rotation "$@"
        ;;
    reads)
        run_reads "$@"
        ;;
    writes)
        run_writes "$@"
        ;;
    mixed)
        run_mixed "$@"
        ;;
    baseline)
        save_baseline "$@"
        ;;
    compare)
        compare_baseline "$@"
        ;;
    clean)
        clean_results
        ;;
    report)
        open_report
        ;;
    flamegraph)
        generate_flamegraph
        ;;
    help|--help|-h)
        show_usage
        ;;
    *)
        print_error "Unknown command: $COMMAND"
        echo ""
        show_usage
        exit 1
        ;;
esac

print_success "Done!"
