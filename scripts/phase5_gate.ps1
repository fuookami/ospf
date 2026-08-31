Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

cargo fmt -p ospf-rust-core -p ospf-rust-framework --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo check -p ospf-rust-core
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo check -p ospf-rust-framework
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test -p ospf-rust-core --lib
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($env:OSPF_CHECK_SCIP_FEATURE -eq "1") {
    cargo check -p ospf-rust-core --features scip
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo check -p ospf-rust-framework --features scip
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo test -p ospf-rust-core --features scip scip
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

if ($env:OSPF_GUROBI_FEATURE) {
    cargo check -p ospf-rust-core --features $env:OSPF_GUROBI_FEATURE
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo check -p ospf-rust-framework --features $env:OSPF_GUROBI_FEATURE
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

powershell -NoProfile -ExecutionPolicy Bypass -File scripts/phase5_scan.ps1
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
