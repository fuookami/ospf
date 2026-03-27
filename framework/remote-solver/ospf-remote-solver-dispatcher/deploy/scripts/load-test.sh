#!/usr/bin/env bash
# Remote Solver Capacity Test Script
#
# This script runs capacity tests and generates reports.
#
# Usage:
#   ./load-test.sh [options]
#
# Options:
#   --config PATH          Configuration file path (default: deploy/config/scheduler.properties)
#   --total-tasks N        Total number of tasks (default: 100)
#   --nodes N              Number of nodes (default: 6)
#   --simple-ratio RATIO   Ratio of simple tasks (default: 0.7)
#   --output FORMAT        Output format: text, json, both (default: text)
#   --output-path PATH     Output file path for JSON report
#   --slo-success-rate R   SLO success rate target (default: 0.999)
#   --slo-throughput R     SLO throughput target (tasks/sec)
#   --help                 Show this help message

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEFAULT_CONFIG="$PROJECT_ROOT/deploy/config/scheduler.properties"

# Default values
CONFIG_PATH="$DEFAULT_CONFIG"
TOTAL_TASKS=100
NODES=6
SIMPLE_RATIO=0.7
OUTPUT_FORMAT="text"
OUTPUT_PATH=""
SLO_SUCCESS_RATE=0.999
SLO_THROUGHPUT=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --config)
            CONFIG_PATH="$2"
            shift 2
            ;;
        --total-tasks)
            TOTAL_TASKS="$2"
            shift 2
            ;;
        --nodes)
            NODES="$2"
            shift 2
            ;;
        --simple-ratio)
            SIMPLE_RATIO="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        --output-path)
            OUTPUT_PATH="$2"
            shift 2
            ;;
        --slo-success-rate)
            SLO_SUCCESS_RATE="$2"
            shift 2
            ;;
        --slo-throughput)
            SLO_THROUGHPUT="$2"
            shift 2
            ;;
        --help)
            sed -n '/^# Usage:/,/^$/p' "$0" | sed 's/^# //'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Validate configuration file
if [[ ! -f "$CONFIG_PATH" ]]; then
    echo "ERROR: Configuration file not found: $CONFIG_PATH"
    exit 1
fi

# Build command arguments
CMD_ARGS=(
    "--config" "$CONFIG_PATH"
    "--total-tasks" "$TOTAL_TASKS"
    "--nodes" "$NODES"
    "--simple-ratio" "$SIMPLE_RATIO"
    "--output" "$OUTPUT_FORMAT"
    "--slo-success-rate" "$SLO_SUCCESS_RATE"
)

if [[ -n "$OUTPUT_PATH" ]]; then
    CMD_ARGS+=("--output-path" "$OUTPUT_PATH")
fi

if [[ -n "$SLO_THROUGHPUT" ]]; then
    CMD_ARGS+=("--slo-throughput" "$SLO_THROUGHPUT")
fi

echo "=== Remote Solver Capacity Test ==="
echo "Configuration: $CONFIG_PATH"
echo "Total tasks: $TOTAL_TASKS"
echo "Nodes: $NODES"
echo "Simple ratio: $SIMPLE_RATIO"
echo "Output format: $OUTPUT_FORMAT"
echo ""

# Run the capacity test
cd "$PROJECT_ROOT"

# Check if Maven is available
if command -v mvn &>/dev/null; then
    echo "Running capacity test via Maven..."
    mvn -q exec:java \
        -Dexec.mainClass="fuookami.ospf.framework.remote_solver.bootstrap.RemoteSolverLoadMain" \
        -Dexec.args="${CMD_ARGS[*]}"
else
    echo "ERROR: Maven not found. Please install Maven or run the test directly."
    exit 1
fi
