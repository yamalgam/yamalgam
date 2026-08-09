#!/usr/bin/env bash
# =============================================================================
# CLI Benchmark Suite using hyperfine
# =============================================================================
#
# End-to-end benchmarks for the CLI binary, measuring real-world performance
# including startup time, argument parsing, and I/O.
#
# Prerequisites:
#   - hyperfine: brew install hyperfine (or cargo install hyperfine)
#
# Usage:
#   ./scripts/bench-cli.sh
#   ./scripts/bench-cli.sh --quick    # fewer runs for faster iteration
#
# Results are saved to bench-reports/
#
# =============================================================================

set -euo pipefail

# Configuration
PACKAGE_NAME="yamalgam"
BINARY_NAME="yg"
RESULTS_DIR="./bench-reports"

# Parse arguments
WARMUP=3
MIN_RUNS=10
if [[ "${1:-}" == "--quick" ]]; then
    WARMUP=1
    MIN_RUNS=3
    echo "Running in quick mode (fewer iterations)"
fi

# Build release binary first
echo "Building release binary..."
cargo build --release --package "$PACKAGE_NAME" --bin "$BINARY_NAME"

BINARY="./target/release/$BINARY_NAME"

# Verify binary exists
if [[ ! -x "$BINARY" ]]; then
    echo "Error: Binary not found at $BINARY"
    exit 1
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

echo ""
echo "=== CLI Benchmark Suite ==="
echo "Binary: $BINARY"
echo "Results: $RESULTS_DIR/"
echo ""

# -----------------------------------------------------------------------------
# Benchmark 1: Version/Info commands (baseline startup time)
# -----------------------------------------------------------------------------
echo "--- Benchmarking: Startup & Info ---"

hyperfine \
    --warmup "$WARMUP" \
    --min-runs "$MIN_RUNS" \
    --export-json "$RESULTS_DIR/hyperfine-startup.json" \
    --export-markdown "$RESULTS_DIR/hyperfine-startup.md" \
    --command-name "version (--version)" "$BINARY --version" \
    --command-name "info" "$BINARY info" \
    --command-name "info --json" "$BINARY info --json" \
    --command-name "help" "$BINARY --help"

echo ""
echo "Startup benchmarks saved to $RESULTS_DIR/hyperfine-startup.*"

# -----------------------------------------------------------------------------
# Benchmark 2: Compare verbose vs non-verbose (if applicable)
# -----------------------------------------------------------------------------
# Uncomment and customize when you have commands that support --verbose:
#
# echo ""
# echo "--- Benchmarking: Verbose Mode ---"
#
# hyperfine \
#     --warmup "$WARMUP" \
#     --min-runs "$MIN_RUNS" \
#     --export-json "$RESULTS_DIR/hyperfine-verbose.json" \
#     --command-name "normal" "$BINARY process input.txt" \
#     --command-name "verbose" "$BINARY --verbose process input.txt"

# -----------------------------------------------------------------------------
# Benchmark 3: Parameterized benchmarks (scaling)
# -----------------------------------------------------------------------------
# Uncomment and customize for commands that accept size parameters:
#
# echo ""
# echo "--- Benchmarking: Scaling ---"
#
# hyperfine \
#     --warmup 2 \
#     --min-runs 5 \
#     --parameter-scan size 100 1000 -D 100 \
#     --export-json "$RESULTS_DIR/hyperfine-scaling.json" \
#     --command-name "size={size}" "$BINARY generate --count {size}"

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "=== Benchmark Complete ==="
echo "Results saved to $RESULTS_DIR/"
echo ""
echo "View results:"
echo "  cat $RESULTS_DIR/hyperfine-startup.md"
echo ""
echo "Compare across runs:"
echo "  hyperfine --warmup 3 '$BINARY info' '$BINARY info --json'"
