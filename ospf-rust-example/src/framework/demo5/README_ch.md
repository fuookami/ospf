# Demo 5：VRPTW Branch-and-Price

[English](README.md)

`framework:demo5` 使用 inline Solomon 风格小夹具接入 Rust network-scheduling crate，覆盖解析、输入适配、Phase I/II 路线编译、ESPPRC 定价和 application Branch-and-Price 主链，不改变已有 `core:demo5` 背包示例语义。

使用以下任一 backend 构建或运行：

```powershell
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --test demo5_scip_bp -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored"
pwsh -NoProfile -Command "cargo run -p ospf-rust-example --features demo5-scip-bp -- framework:demo5"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --test demo5_gurobi_bp -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored"
pwsh -NoProfile -Command "cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored"
pwsh -NoProfile -Command "cargo run -p ospf-rust-example --features demo5-gurobi-bp -- framework:demo5"
```

解析器支持标准 Solomon `VEHICLE` 与 `CUSTOMER` 段，校验七列节点数据，保留最后一条记录，并返回带行号的双语错误。Demo17 夹具提供 25 客户和 100 客户实例。integration target 只执行自身的 parser/adapter 与 target harness；上面的 `--lib` 命令才会执行 25 客户 direct-MIP 交叉验证、100 客户 smoke、strict-proof fixture 和 SCIP 合法终态 validator 门禁。当前 direct-MIP oracle 仅支持单一车辆类型。native solver 测试默认标为 ignored，必须使用 `-- --include-ignored` 显式执行；本机缺少对应求解器或许可证时，门禁会失败，不会空通过。
