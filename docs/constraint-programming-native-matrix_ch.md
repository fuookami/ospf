# CP 原生能力矩阵

本文记录 Rust CP 实现的证据边界，并与 MIP-backed CP facade 及统一 solver
report 合同分开。原生探针通过不代表 MIP-backed facade 已变成原生 CP backend。

## 绑定与版本

首版目标为 `russcip 0.9.1` 和 `scip-sys 0.1.26`。探针位于
`ospf-rust-core/tests/scip_cp_probe_integration.rs`，必须显式启用 `scip`、
`scip-bundled` 或 `scip-from-source` feature 执行。仅 feature 编译不构成
native 证据。

| 能力 | 证据 | 实际状态 |
| --- | --- | --- |
| safe 线性整数/布尔变量 | `russcip` model API 与探针 | 探针子集 Native |
| indicator 约束 | safe `add_cons_indicator` 与探针 | 正 literal、`<=` 行 Native |
| SOS1 | safe `add_cons_sos1` 与探针 | Native |
| 状态、incumbent、best bound | `Status`、`WithSolutions`、`WithSolvingStats` 与探针 | Native 元数据 |
| 取消 | 项目 `SolveHandle`、SCIP event bridge 与探针 | Native interrupt bridge |
| event handler | `Eventhdlr` 探针 | Native，但仅限求解作用域 |
| probing | `Prober` 探针与 `SCIPinProbing` 后置条件 | Native，但仅限临时节点作用域 |
| 负 literal、Boolean AND/OR/XOR、full reification | CP AST evaluator、穷举 formulation oracle 和严格有限 lowerer | `ExactLowering`，不宣称 SCIP native |
| sparse domain、AllDifferent、Element、Allowed/Forbidden table | CP snapshot 校验与穷举 lowerer 测试 | 有限且受预算约束时 `ExactLowering` |
| mandatory/optional、fixed/variable-duration NoOverlap | 严格有限 lowerer、源 snapshot 复验和 Gurobi/SCIP facade 测试 | 所有边界有限时 `ExactLowering` |
| Circuit、Automaton、Reservoir | 逐约束 support analysis 与结构化 lowering error | generic production lowering 中 `Unsupported` |
| 真正增量 CP session | `russcip 0.9.1` 没有公开合同 | Unsupported，使用 snapshot rebuild |
| optional/variable-duration 原生 interval | 没有 safe binding 合同 | Unsupported |
| Cumulative | 仅 raw C API 探针 | Conditional 研究路径，未进入生产 |
| CP 原生 conflict graph/IIS | crate 未使用 CP 层公开合同 | Unsupported，使用 verified rebuild/deletion fallback |
| 统一 report/proof/cancellation/provenance | `ScipConstraintProgrammingSolver` 与共享 `SolveReport<i64>` 合同测试 | ExactLowering facade，不宣称 native CP |

## CP 交付边界

当某项能力已经实现并复验，或已经明确冻结为 `Conditional`/`Unsupported` 时，Rust CP
声明范围即完成。因此生产 SCIP 入口是严格有限 MIP-backed facade，而不是通用 native
SCIP/CIP CP engine。backend 探针、精确 lowering、verified conflict 重建、remote
materialization、portable checkpoint rebuild 和 Gantt differential 均由各自专用源码与
测试覆盖。缺少 native library、许可证、bundled 下载或 source build 时记为
`not executed/unsupported`；只编译不能把结果升级为 Native。

## raw cumulative 安全边界

探针在同一作用域调用 `SCIPcreateConsBasicCumulative` 和 `SCIPaddCons`。数组只在调用期间可写，SCIP 变量仍由 model 所有。原始 constraint 加入模型后由 model owner 执行最终释放；探针特意不再调用 `SCIPreleaseCons`，因为 `russcip 0.9.1` 在求解时可能已经建立 transformed constraint，重复释放会被 SCIP 拒绝。raw 路径由 `scip-sys` 依赖版本门禁，只有专用 wrapper
补齐返回码转换、flag 配置、ownership、溢出/时间 horizon 校验以及成功/失败测试后，
才允许进入生产 CP facade。

当前 fallback 仍是严格 MIP lowering 的已支持子集；Cumulative/Circuit/Automaton/
Reservoir 在 generic MIP lowerer 中继续结构化返回 `Unsupported`。不能把 raw 探针
结果写成生产 native 能力。

## probing 作用域

event handler 只有在 SCIP `Solving` 阶段才能创建临时 probing。对象析构结束 probing，
测试检查 `SCIPinProbing == 0`。probing 修改只对该次搜索作用域有效，不是可复用的增量
model 或 assumption session，因此 CP session 继续使用 snapshot rebuild。
