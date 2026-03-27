#!/usr/bin/env bash
# Remote Solver Alert Linkage Validation Script
#
# This script validates that alert configuration files are correctly formatted
# and that alert rules properly reference defined metrics.
#
# Usage:
#   ./validate-alert-linkage.sh [--strict]
#
# Options:
#   --strict  Exit with error if any warnings are found

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OBSERVABILITY_DIR="$PROJECT_ROOT/deploy/observability"

STRICT_MODE=false
if [[ "${1:-}" == "--strict" ]]; then
    STRICT_MODE=true
fi

echo "=== Remote Solver Alert Linkage Validation ==="
echo "Project root: $PROJECT_ROOT"
echo ""

ERRORS=0
WARNINGS=0

# Check 1: Prometheus alert rules file exists and is valid YAML
echo "[1/5] Checking Prometheus alert rules..."
PROMETHEUS_RULES="$OBSERVABILITY_DIR/prometheus/alerts-remote-solver.yml"
if [[ ! -f "$PROMETHEUS_RULES" ]]; then
    echo "ERROR: Prometheus alert rules file not found: $PROMETHEUS_RULES"
    ((ERRORS++))
else
    echo "  Found: $PROMETHEUS_RULES"

    # Validate YAML syntax
    if command -v python3 &>/dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$PROMETHEUS_RULES'))" 2>/dev/null; then
            echo "  YAML syntax: OK"
        else
            # Check if yaml module is available
            if ! python3 -c "import yaml" 2>/dev/null; then
                echo "  WARNING: PyYAML module not available, skipping YAML validation"
                ((WARNINGS++))
            else
                echo "ERROR: Invalid YAML syntax in $PROMETHEUS_RULES"
                ((ERRORS++))
            fi
        fi
    else
        echo "  WARNING: python3 not available, skipping YAML validation"
        ((WARNINGS++))
    fi

    # Check required alert names
    REQUIRED_ALERTS=(
        "RemoteSolverHighFailureRate"
        "RemoteSolverSliceTimeoutSpike"
        "RemoteSolverNodeTimeoutRecoverySpike"
        "RemoteSolverCostSpike"
    )
    for alert_name in "${REQUIRED_ALERTS[@]}"; do
        if grep -q "alert: $alert_name" "$PROMETHEUS_RULES"; then
            echo "  Alert '$alert_name': Found"
        else
            echo "WARNING: Alert '$alert_name' not found in rules file"
            ((WARNINGS++))
        fi
    done
fi

echo ""

# Check 2: AlertManager configuration file exists and is valid YAML
echo "[2/5] Checking AlertManager configuration..."
ALERTMANAGER_CONFIG="$OBSERVABILITY_DIR/alertmanager/alertmanager.yml"
if [[ ! -f "$ALERTMANAGER_CONFIG" ]]; then
    echo "ERROR: AlertManager config file not found: $ALERTMANAGER_CONFIG"
    ((ERRORS++))
else
    echo "  Found: $ALERTMANAGER_CONFIG"

    # Validate YAML syntax
    if command -v python3 &>/dev/null; then
        if python3 -c "import yaml; yaml.safe_load(open('$ALERTMANAGER_CONFIG'))" 2>/dev/null; then
            echo "  YAML syntax: OK"
        else
            # Check if yaml module is available
            if ! python3 -c "import yaml" 2>/dev/null; then
                echo "  WARNING: PyYAML module not available, skipping YAML validation"
                ((WARNINGS++))
            else
                echo "ERROR: Invalid YAML syntax in $ALERTMANAGER_CONFIG"
                ((ERRORS++))
            fi
        fi
    else
        echo "  WARNING: python3 not available, skipping YAML validation"
        ((WARNINGS++))
    fi

    # Check required receivers
    REQUIRED_RECEIVERS=("default-receiver" "critical-receiver" "warning-receiver")
    for receiver in "${REQUIRED_RECEIVERS[@]}"; do
        if grep -q "name: '$receiver'" "$ALERTMANAGER_CONFIG" || grep -q "name: \"$receiver\"" "$ALERTMANAGER_CONFIG"; then
            echo "  Receiver '$receiver': Found"
        else
            echo "WARNING: Receiver '$receiver' not found in AlertManager config"
            ((WARNINGS++))
        fi
    done
fi

echo ""

# Check 3: Metrics spec file exists
echo "[3/5] Checking metrics specification..."
METRICS_SPEC="$PROJECT_ROOT/docs/observability/metrics-spec.md"
if [[ ! -f "$METRICS_SPEC" ]]; then
    echo "WARNING: Metrics spec file not found: $METRICS_SPEC"
    ((WARNINGS++))
else
    echo "  Found: $METRICS_SPEC"
fi

echo ""

# Check 4: Alert-to-metric mapping
echo "[4/5] Checking alert-to-metric mapping..."
if [[ -f "$PROMETHEUS_RULES" && -f "$METRICS_SPEC" ]]; then
    # Extract metric names used in alerts
    ALERT_METRICS=$(grep -oE 'remote_solver_[a-z_]+' "$PROMETHEUS_RULES" | sort -u)

    # Check if metrics are documented
    while IFS= read -r metric; do
        if grep -q "$metric" "$METRICS_SPEC"; then
            echo "  Metric '$metric': Documented"
        else
            echo "WARNING: Metric '$metric' used in alert but not documented in metrics-spec.md"
            ((WARNINGS++))
        fi
    done <<< "$ALERT_METRICS"
else
    echo "  Skipped: Missing files"
fi

echo ""

# Check 5: Grafana dashboard exists
echo "[5/5] Checking Grafana dashboard..."
GRAFANA_DASHBOARD="$OBSERVABILITY_DIR/grafana/remote-solver-overview.json"
if [[ ! -f "$GRAFANA_DASHBOARD" ]]; then
    echo "WARNING: Grafana dashboard not found: $GRAFANA_DASHBOARD"
    ((WARNINGS++))
else
    echo "  Found: $GRAFANA_DASHBOARD"

    # Check if dashboard references key metrics
    if grep -q "remote_solver_task" "$GRAFANA_DASHBOARD"; then
        echo "  Task metrics: Referenced"
    else
        echo "WARNING: Grafana dashboard does not reference task metrics"
        ((WARNINGS++))
    fi
fi

echo ""
echo "=== Validation Summary ==="
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [[ $ERRORS -gt 0 ]]; then
    echo "FAIL: Alert linkage validation failed with $ERRORS error(s)"
    exit 1
fi

if [[ "$STRICT_MODE" == true && $WARNINGS -gt 0 ]]; then
    echo "FAIL: Strict mode enabled, validation failed with $WARNINGS warning(s)"
    exit 1
fi

echo "PASS: Alert linkage validation completed successfully"
exit 0