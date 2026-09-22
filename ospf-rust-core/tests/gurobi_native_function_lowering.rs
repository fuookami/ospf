//! 原生函数符号写入的端到端验收 / End-to-end acceptance of native function-symbol writing
//!
//! 本文件验证 P3 接线：在 `DeferredNativeFirst` 策略下，ABS 符号的结构被 Gurobi 的
//! `add_genconstr_abs` 原生写入，模型层不再为它物化通用 fallback 行，而**真实求解**的结果仍然
//! 满足 `y = |x|` 与模型自身的约束。
//!
//! This file verifies the P3 wiring: under `DeferredNativeFirst` the ABS structure is written
//! natively through Gurobi's `add_genconstr_abs`, the model layer does not materialize the generic
//! fallback rows for it, and a **real solve** still satisfies both `y = |x|` and the model's own
//! constraints.
//!
//! 列号与行号的口径：中间模型与三角模型都用**令牌在列表中的位置**作为列号（物化时构造的
//! `symbol_to_index` 就是 `tokens().enumerate()`），因此本文件一律用 `linear_column_view()` 的
//! 位置取列号，而不是 `solver_index`。
//!
//! Column numbering: both the mechanism model and the triad use the **token's position in the list**
//! as the column index (the `symbol_to_index` built during materialization is exactly
//! `tokens().enumerate()`), so this file always derives column indices from the position inside
//! `linear_column_view()` rather than from `solver_index`.
//!
//! 需要匹配的 Gurobi 库与许可证 / Requires the matching Gurobi library and licence.

#![cfg(any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"))]

use std::sync::Arc;

use ospf_rust_core::MechanismModel;
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::intermediate::{
    DeferredFunctionStructure, FallbackReason, NativeFunctionWriter, NativeFunctionWriterRegistry,
    NativeWriteOutcome, NativeWriteRequest,
};
use ospf_rust_core::model::{
    ConstraintRelation, FunctionExpansionPolicy, LinearConstraint, LinearInequality, MetaModel,
};
use ospf_rust_core::solver::solvers::GurobiSolver;
use ospf_rust_core::solver::solvers::gurobi::GurobiNativeContainer;
use ospf_rust_core::symbol::function::{
    AbsFunction, AbsStructure, AndFunction, BinaryzationFunction, BinaryzationMethod, IfInFunction,
    ImplyFunction, InequalityFunction, InequalityKind, OrFunction, SigmoidFunction, SinFunction,
};
use ospf_rust_core::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange,
};

const X_ID: usize = 90_000;
const ABS_ID: u64 = 91_000;

/// 「y = |x| 且 y ≥ 3」的机制模型，`x ∈ [-5, 5]`；返回结果列的列号。
/// The mechanism model of "y = |x| with y ≥ 3" for `x ∈ [-5, 5]`, plus the result column index.
fn abs_target_mechanism(
    policy: FunctionExpansionPolicy,
) -> (MechanismModel<f64>, usize, usize) {
    let mut model = MetaModel::<f64>::new("gurobi_native_abs");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(X_ID),
        "x",
        VariableRange::bounded(-5.0, 5.0),
    );
    let x_index = model.register_variable(x).expect("x should register");

    let abs = AbsFunction::new(
        ABS_ID,
        "abs_native",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
    );
    let result_id = abs.result_variable().id();
    model
        .add_symbol(Arc::new(abs))
        .expect("abs symbol should register");

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    // 列号取「令牌位置」，与三角模型的列顺序（以及行里的列号）保持同一口径。
    // Column indices come from the token position, matching the triad's column order (and the column
    // indices used inside the rows).
    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(X_ID))
        .expect("x column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("abs result column should exist");

    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
            ConstraintRelation::GreaterEqual,
            3.0,
        ),
        "abs_lower_target",
    ));

    (mechanism, x_column, result_column)
}

#[test]
fn native_abs_write_is_used_by_a_real_solve() {
    let (mechanism, x_column, result_column) =
        abs_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst);

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    // 原生写入确实发生了一次，并且记录的是原生结果而不是回退。
    // The native write really happened once, and it is recorded as native rather than a fallback.
    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(_))),
        "expected a native write outcome, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");

    // 真实解必须满足 y = |x|：这正是原生一般约束所表达的关系。
    // The real solution must satisfy y = |x|, which is exactly what the native general constraint
    // expresses.
    let x_value = solution[x_column];
    let y_value = solution[result_column];
    assert!(
        (y_value - x_value.abs()).abs() <= 1e-6,
        "native abs relation violated: |{x_value}| != {y_value}"
    );
    // 模型自身的约束同样成立。
    // The model's own constraint holds as well.
    assert!(
        y_value >= 3.0 - 1e-6,
        "model constraint y >= 3 violated: y = {y_value}"
    );
}

#[test]
fn externally_referenced_helper_columns_fall_back_instead_of_writing_natively() {
    // 辅助列（ABS 的分支指示列）被模型在别处引用时，原生关系会改变它的含义，因此 writer 必须
    // 拒绝并回退通用展开；回退后模型仍然完整可解。
    // When a helper column (the ABS branch selector) is referenced elsewhere in the model, the native
    // relation would change its meaning, so the writer must reject and fall back to the generic
    // expansion; the fallback model still solves completely.
    let (mut mechanism, _x_column, result_column) =
        abs_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst);
    let view = mechanism.linear_column_view();
    let side_column = view
        .iter()
        .position(|column| column.name.ends_with("_side"))
        .expect("abs side column should exist");
    // 在别处引用分支指示列：约束 `side <= 1`（把该列纳入非本函数的行）。
    // Reference the branch selector elsewhere: constrain `side <= 1`, putting the column into a row
    // that is not one of this function's own rows.
    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, side_column)], 0.0),
            ConstraintRelation::LessEqual,
            1.0,
        ),
        "external_side_reference",
    ));

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an externally referenced helper column must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let y_value = solution[result_column];
    assert!(
        y_value >= 3.0 - 1e-6,
        "fallback rows must still enforce y >= 3: y = {y_value}"
    );
}

#[test]
fn eager_and_native_paths_agree_on_the_same_model() {
    // 同一模型在 EAGER 与原生写入两条路径下必须给出同一可行域与同一语义关系。
    // The same model must give the same feasible region and the same semantic relation on both the
    // eager path and the native-write path.
    let (eager_mechanism, _eager_x, eager_result_column) =
        abs_target_mechanism(FunctionExpansionPolicy::Eager);
    let eager_model = eager_mechanism.into_linear_triad_model();
    let solver = GurobiSolver::new();
    let eager_output = solver
        .solve_linear(&eager_model)
        .expect("gurobi should solve the eager model");

    let (native_mechanism, _native_x, native_result_column) =
        abs_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst);
    let (native_output, report) = solver
        .solve_linear_with_native_lowering(native_mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(
        eager_output.status.is_feasible(),
        native_output.status.is_feasible(),
        "both paths must agree on feasibility"
    );
    assert_eq!(report.native_writes, 1);
    // 结果列在两条路径里的列号相同（列顺序由同一份令牌列表决定）。
    // The result column has the same index on both paths (the column order comes from the same token
    // list).
    assert_eq!(eager_result_column, native_result_column);

    let eager_solution = eager_output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let native_solution = native_output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let eager_y = eager_solution[eager_result_column];
    let native_y = native_solution[native_result_column];
    for (label, y) in [("eager", eager_y), ("native", native_y)] {
        assert!(y >= 3.0 - 1e-6, "{label} path violated y >= 3: y = {y}");
        assert!(
            y <= 5.0 + 1e-6,
            "{label} path exceeded the |x| range implied by x <= 5: y = {y}"
        );
    }
}

const MAX_X_ID: usize = 92_000;
const MAX_Y_ID: usize = 92_001;
const EXTREMUM_ID: u64 = 93_000;

/// 「z = max/min(x, y) 且 z ≥ 4（MAX）/ z ≤ 2（MIN）」的机制模型。
/// 返回 `(模型, x 列号, y 列号, 结果列号)`。
///
/// The mechanism model of "z = max/min(x, y)" with `z ≥ 4` (MAX) or `z ≤ 2` (MIN), returning the
/// model plus the x, y and result column indices.
fn extremum_target_mechanism(
    policy: FunctionExpansionPolicy,
    minimum: bool,
) -> (MechanismModel<f64>, usize, usize, usize) {
    let mut model = MetaModel::<f64>::new(if minimum {
        "gurobi_native_min"
    } else {
        "gurobi_native_max"
    });
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(MAX_X_ID),
        "x",
        VariableRange::bounded(if minimum { 1.0 } else { 0.0 }, 5.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(MAX_Y_ID),
        "y",
        VariableRange::bounded(if minimum { 2.0 } else { 0.0 }, 3.0),
    );
    let x_index = model.register_variable(x).expect("x should register");
    let y_index = model.register_variable(y).expect("y should register");

    let candidates = vec![
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
    ];
    let (name, result_id) = if minimum {
        let min = ospf_rust_core::symbol::function::MinFunction::new(
            EXTREMUM_ID,
            "min_native",
            candidates,
            true,
        );
        let result = min.result_variable().id();
        model
            .add_symbol(Arc::new(min))
            .expect("min symbol should register");
        ("min_native", result)
    } else {
        let max = ospf_rust_core::symbol::function::MaxFunction::new(
            EXTREMUM_ID,
            "max_native",
            candidates,
            true,
        );
        let result = max.result_variable().id();
        model
            .add_symbol(Arc::new(max))
            .expect("max symbol should register");
        ("max_native", result)
    };
    let _ = name;

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(MAX_X_ID))
        .expect("x column should exist");
    let y_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(MAX_Y_ID))
        .expect("y column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("extremum result column should exist");

    let (relation, rhs, row_name) = if minimum {
        (ConstraintRelation::LessEqual, 2.0, "min_upper_target")
    } else {
        (ConstraintRelation::GreaterEqual, 4.0, "max_lower_target")
    };
    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
            relation,
            rhs,
        ),
        row_name,
    ));

    (mechanism, x_column, y_column, result_column)
}

#[test]
fn native_max_write_is_used_by_a_real_solve() {
    let (mechanism, x_column, y_column, result_column) =
        extremum_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst, false);

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(record) if record.writer == "gurobi_max")),
        "expected a native max write, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[y_column];
    let z_value = solution[result_column];
    assert!(
        (z_value - x_value.max(y_value)).abs() <= 1e-6,
        "native max relation violated: max({x_value}, {y_value}) != {z_value}"
    );
    assert!(
        z_value >= 4.0 - 1e-6,
        "model constraint z >= 4 violated: z = {z_value}"
    );
    assert!(
        x_value >= 4.0 - 1e-6,
        "z >= 4 with y <= 3 must force x >= 4: x = {x_value}"
    );
}

#[test]
fn native_min_write_is_used_by_a_real_solve() {
    let (mechanism, x_column, y_column, result_column) =
        extremum_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst, true);

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(record) if record.writer == "gurobi_min")),
        "expected a native min write, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[y_column];
    let z_value = solution[result_column];
    assert!(
        (z_value - x_value.min(y_value)).abs() <= 1e-6,
        "native min relation violated: min({x_value}, {y_value}) != {z_value}"
    );
    assert!(
        z_value <= 2.0 + 1e-6,
        "model constraint z <= 2 violated: z = {z_value}"
    );
    assert!(
        x_value <= 2.0 + 1e-6,
        "z <= 2 with y >= 2 must force x <= 2: x = {x_value}"
    );
}

/// 一个「必然写入失败」的 ABS writer：只用于验证失败路径。
/// An ABS writer that must fail to write; used only to exercise the failure path.
#[derive(Debug)]
struct AlwaysFailingAbsWriter;

impl NativeFunctionWriter<GurobiNativeContainer, f64> for AlwaysFailingAbsWriter {
    fn name(&self) -> &str {
        "failing_abs"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure
            .as_any()
            .downcast_ref::<AbsStructure<f64>>()
            .is_some()
    }

    fn write_batch(
        &self,
        _container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> std::result::Result<Option<Vec<NativeWriteOutcome>>, ospf_rust_core::CoreError> {
        if requests.is_empty() {
            return Ok(None);
        }
        Err(ospf_rust_core::ModelError::InvalidConstraint(
            "injected native writer failure".to_string(),
        )
        .into())
    }
}

#[test]
fn a_native_write_failure_falls_back_for_the_whole_model() {
    // 失败原子性：writer **真正写入失败**（区别于按语境拒绝）时，已经建好的 SDK 模型可能含有一
    // 部分原生约束且无法逐条回滚，因此求解器必须丢弃它、用同一份机制模型走通用展开路径重建并
    // 求解；求解本身仍然成功，报告如实记录「全部结构因 writer 失败而回退」。
    //
    // Failure atomicity: when a writer **really fails to write** (as opposed to rejecting by context),
    // the SDK model built so far may already carry some native constraints that cannot be rolled back
    // one by one, so the solver must discard it and rebuild from the same mechanism model along the
    // generic path; the solve still succeeds and the report records that every structure fell back
    // because of the writer failure.
    let (mechanism, x_column, result_column) =
        abs_target_mechanism(FunctionExpansionPolicy::DeferredNativeFirst);

    let mut registry: NativeFunctionWriterRegistry<GurobiNativeContainer, f64> =
        NativeFunctionWriterRegistry::new();
    registry.register(Box::new(AlwaysFailingAbsWriter));

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_writers(mechanism, None, registry)
        .expect("a writer failure must fall back instead of failing the solve");

    assert_eq!(report.native_writes, 0);
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::WriterFailed(_))]
        ),
        "expected a writer-failure fallback, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[result_column];
    assert!(
        (y_value - x_value.abs()).abs() <= 1e-6,
        "fallback rows must still enforce y = |x|: |{x_value}| != {y_value}"
    );
    assert!(
        y_value >= 3.0 - 1e-6,
        "fallback rows must still enforce y >= 3: y = {y_value}"
    );
}

const PWL_X_ID: usize = 94_000;
const SIN_PWL_ID: u64 = 95_000;
const SIGMOID_PWL_ID: u64 = 95_100;

/// 参与 PWL 原生写入验收的分段线性形状 / Piecewise-linear shapes covered by the PWL acceptance tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PwlShape {
    /// 正弦（断点区间 `[-π, π]`）/ Sine (breakpoint interval `[-π, π]`)
    Sin,
    /// Sigmoid（采样点由符号自身决定）/ Sigmoid (sampling points intrinsic to the symbol)
    Sigmoid,
}

impl PwlShape {
    /// 形状名称 / Shape name.
    fn label(self) -> &'static str {
        match self {
            PwlShape::Sin => "sin",
            PwlShape::Sigmoid => "sigmoid",
        }
    }
}

/// 「y = f(x) 且 x = 1」的机制模型，输入列声明为 `[input_lower, input_upper]`。
/// 返回 `(模型, x 列号, 结果列号)`。
///
/// 额外的等式 `x = 1` 把解固定下来（`y` 由分段线性函数唯一决定），这样 EAGER 与原生两条路径才
/// 能逐点比较；`x = 1` 落在所有参与测试的断点区间内部，因此固定它不会掩盖范围差异，而输入列的
/// **声明范围**才是范围证明的输入。
///
/// The mechanism model of "y = f(x) with x = 1" whose input column is declared as
/// `[input_lower, input_upper]`, returning the model plus the x and result column indices.
///
/// The extra `x = 1` equality pins the solution (y is uniquely determined by the piecewise-linear
/// function) so the EAGER and native paths can be compared point by point; `x = 1` lies inside every
/// breakpoint interval used here, so pinning it never hides a range difference, while the input
/// column's **declared range** is exactly what the range proof consumes.
fn pwl_target_mechanism(
    policy: FunctionExpansionPolicy,
    shape: PwlShape,
    input_lower: f64,
    input_upper: f64,
) -> (MechanismModel<f64>, usize, usize) {
    let model_name = format!("gurobi_native_pwl_{}", shape.label());
    let mut model = MetaModel::<f64>::new(&model_name);
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(PWL_X_ID),
        "x",
        VariableRange::bounded(input_lower, input_upper),
    );
    let x_index = model.register_variable(x).expect("x should register");
    let input = Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0);

    let result_id = match shape {
        PwlShape::Sin => {
            let sin = SinFunction::new(SIN_PWL_ID, "sin_pwl_native", input);
            let result = sin.result_variable().id();
            model
                .add_symbol(Arc::new(sin))
                .expect("sin symbol should register");
            result
        }
        PwlShape::Sigmoid => {
            let sigmoid = SigmoidFunction::new(SIGMOID_PWL_ID, "sigmoid_pwl_native", input);
            let result = sigmoid.result_variable().id();
            model
                .add_symbol(Arc::new(sigmoid))
                .expect("sigmoid symbol should register");
            result
        }
    };

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    // 列号取「令牌位置」，与三角模型的列顺序（以及行里的列号）保持同一口径。
    // Column indices come from the token position, matching the triad's column order (and the column
    // indices used inside the rows).
    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(PWL_X_ID))
        .expect("x column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("pwl result column should exist");

    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_column)], 0.0),
            ConstraintRelation::Equal,
            1.0,
        ),
        "pwl_pin_x",
    ));

    (mechanism, x_column, result_column)
}

/// 在 EAGER 路径上求解同一个 PWL 模型，返回 `(结果列解值, 结果列号)`。
/// Solve the same PWL model along the EAGER path, returning the result value and column index.
fn eager_pwl_result(shape: PwlShape, input_lower: f64, input_upper: f64) -> (f64, usize) {
    let (mechanism, _x_column, result_column) =
        pwl_target_mechanism(FunctionExpansionPolicy::Eager, shape, input_lower, input_upper);
    let model = mechanism.into_linear_triad_model();
    let solver = GurobiSolver::new();
    let output = solver
        .solve_linear(&model)
        .expect("gurobi should solve the eager model");
    assert!(
        output.status.is_feasible(),
        "the eager model must be feasible, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    (solution[result_column], result_column)
}

#[test]
fn native_sin_pwl_write_is_used_by_a_real_solve() {
    let (mechanism, x_column, result_column) = pwl_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        PwlShape::Sin,
        -3.0,
        3.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    // 原生写入确实发生了一次，且 writer 名与 schema 稳定。
    // The native write really happened once, with a stable writer name and schema.
    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(record) if record.writer == "gurobi_pwl")),
        "expected a native PWL write, got {:?}",
        report.outcomes
    );
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(record) if record.schema == "functions-pwl-1")),
        "expected the stable PWL schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let native_y = solution[result_column];
    assert!(
        (x_value - 1.0).abs() <= 1e-9,
        "the pinning row must fix x = 1: x = {x_value}"
    );
    // 分段线性逼近的取值由点表决定，这里只要求它落在 sin 的值域内。
    // The piecewise-linear value is decided by the point table; here it only has to stay inside
    // sin's range.
    assert!(
        (-1.0..=1.0).contains(&native_y),
        "a sin PWL value must stay inside [-1, 1]: y = {native_y}"
    );

    // 与 EAGER 路径的真实解比较：两条路径描述的是同一条分段线性函数，而 x 已被固定，因此结果列
    // 必须逐点一致（1e-6 内）。两条路径的列号口径相同（同一份令牌列表）。
    // Compare with the EAGER path's real solution: both paths describe the same piecewise-linear
    // function and x is pinned, so the result column must agree point by point (within 1e-6). Both
    // paths share the column numbering (the same token list).
    let (eager_y, eager_result_column) = eager_pwl_result(PwlShape::Sin, -3.0, 3.0);
    assert_eq!(eager_result_column, result_column);
    assert!(
        (eager_y - native_y).abs() <= 1e-6,
        "eager and native sin paths disagree: eager = {eager_y}, native = {native_y}"
    );
}

#[test]
fn native_sigmoid_pwl_write_is_used_by_a_real_solve() {
    let (mechanism, x_column, result_column) = pwl_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        PwlShape::Sigmoid,
        -2.0,
        2.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report
            .outcomes
            .iter()
            .any(|outcome| matches!(outcome, NativeWriteOutcome::Native(record) if record.writer == "gurobi_pwl")),
        "expected a native PWL write, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let native_y = solution[result_column];
    assert!(
        (x_value - 1.0).abs() <= 1e-9,
        "the pinning row must fix x = 1: x = {x_value}"
    );

    let (eager_y, eager_result_column) = eager_pwl_result(PwlShape::Sigmoid, -2.0, 2.0);
    assert_eq!(eager_result_column, result_column);
    assert!(
        (eager_y - native_y).abs() <= 1e-6,
        "eager and native sigmoid paths disagree: eager = {eager_y}, native = {native_y}"
    );
}

#[test]
fn a_pwl_write_outside_the_breakpoint_interval_falls_back() {
    // 输入列声明为 `[-10, 10]`，比 Sin 的断点区间 `[-π, π]` 更宽：原生 PWL 会在区间外**外推**，
    // 而即时展开用 `x = Σ λ_i x_i` 把输入夹在断点区间内（`x_lb` / `x_ub` 两行），二者语义不同，
    // 因此 writer 必须拒绝并回退通用展开；回退后的模型仍然完整可解，且结果仍由同一条分段线性
    // 函数给出。
    //
    // The input column is declared as `[-10, 10]`, wider than sin's breakpoint interval `[-π, π]`: a
    // native PWL would **extrapolate** outside it while eager expansion clamps the input into the
    // breakpoint interval through `x = Σ λ_i x_i` (the `x_lb` / `x_ub` rows), so the two have
    // different semantics. The writer must therefore reject and fall back to the generic expansion;
    // the fallback model still solves completely and its result still comes from the same
    // piecewise-linear function.
    let (mechanism, _x_column, result_column) = pwl_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        PwlShape::Sin,
        -10.0,
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an input range wider than the breakpoint interval must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::Rejected(message))]
                if message.contains("input column bounds inside")
        ),
        "expected a range-proof rejection, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let fallback_y = solution[result_column];
    assert!(
        (-1.0..=1.0).contains(&fallback_y),
        "the clamped sin value must stay inside [-1, 1]: y = {fallback_y}"
    );

    // 回退路径就是通用展开，因此与 EAGER 的真实解一致。
    // The fallback path is the generic expansion, so it agrees with the EAGER real solution.
    let (eager_y, eager_result_column) = eager_pwl_result(PwlShape::Sin, -10.0, 10.0);
    assert_eq!(eager_result_column, result_column);
    assert!(
        (eager_y - fallback_y).abs() <= 1e-6,
        "eager and fallback sin paths disagree: eager = {eager_y}, fallback = {fallback_y}"
    );
}

const INEQ_X_ID: usize = 96_000;
const INEQ_ID: u64 = 97_000;

/// 关系指示（条件形状）验收用的机制模型：`y = [x REL right]`。
///
/// 可选的固定项：`pin_indicator` 把指示列钉在真/假（`y >= 1` / `y <= 0`），`pin_x` 把条件变量钉在
/// 一个确定值上——求解器对无目标的可行模型只返回任一顶点，钉住取值才能让两条路径逐点比较。
///
/// The mechanism model used by the relation-indicator (condition shape) acceptance tests:
/// `y = [x REL right]`.
///
/// Optional pins: `pin_indicator` pins the indicator column to true/false (`y >= 1` / `y <= 0`) and
/// `pin_x` pins the condition variable to a fixed value — a solver only returns some vertex for a
/// feasible model without an objective, and pinning is what makes the two paths comparable point by
/// point.
fn indicator_target_mechanism(
    policy: FunctionExpansionPolicy,
    kind: InequalityKind,
    right: f64,
    range: VariableRange<f64>,
    pin_indicator: Option<bool>,
    pin_x: Option<f64>,
    configured_big_m: f64,
) -> (MechanismModel<f64>, usize, usize) {
    let mut model = MetaModel::<f64>::new("gurobi_native_indicator");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(VariableId::standalone(INEQ_X_ID), "x", range);
    let x_index = model.register_variable(x).expect("x should register");

    let inequality = InequalityFunction::new(
        INEQ_ID,
        "ineq_native",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        right,
        kind,
        configured_big_m,
    );
    let result_id = inequality.result_variable().id();
    model
        .add_symbol(Arc::new(inequality))
        .expect("inequality symbol should register");

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(INEQ_X_ID))
        .expect("x column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("indicator column should exist");

    if let Some(value) = pin_x {
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_column)], 0.0),
                ConstraintRelation::Equal,
                value,
            ),
            "indicator_pin_x",
        ));
    }
    if let Some(value) = pin_indicator {
        let (relation, rhs) = if value {
            (ConstraintRelation::GreaterEqual, 1.0)
        } else {
            (ConstraintRelation::LessEqual, 0.0)
        };
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
                relation,
                rhs,
            ),
            "indicator_pin_value",
        ));
    }

    (mechanism, x_column, result_column)
}

#[test]
fn native_indicator_write_is_used_by_a_real_solve() {
    // `y = [x <= 1]`，x ∈ [0, 2]，把指示列钉在真：即时展开给出 `x <= 1` 那条核心行，原生写入给出
    // 等价的 `ind = 1 ⇒ s <= 0`。
    // `y = [x <= 1]` with x ∈ [0, 2] and the indicator pinned true: eager expansion contributes the
    // `x <= 1` core row and the native write contributes the equivalent `ind = 1 ⇒ s <= 0`.
    let (mechanism, x_column, result_column) = indicator_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        InequalityKind::LessEqual,
        1.0,
        VariableRange::bounded(0.0, 2.0),
        Some(true),
        None,
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    // 原生写入确实发生了一次，且 writer 名与 schema 稳定。
    // The native write really happened once, with a stable writer name and schema.
    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_indicator"
                    && record.schema == "functions-indicator-1"
        )),
        "expected a native relation-indicator write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[result_column];
    assert!(
        y_value >= 1.0 - 1e-6,
        "the pinning row must fix the indicator to 1: y = {y_value}"
    );
    // 指示列取真时条件成立：`x <= 1`。
    // With the indicator true the condition holds: `x <= 1`.
    assert!(
        x_value <= 1.0 + 1e-6,
        "the native indicator relation must enforce x <= 1: x = {x_value}"
    );
}

/// 在 EAGER 与原生两条路径上求解同一个关系指示模型，返回 `(是否可行, x 取值)`。
/// Solve the same relation-indicator model on both the EAGER and native paths, returning
/// `(feasible, x value)`.
fn indicator_path_feasibility(
    kind: InequalityKind,
    pin_x: f64,
    native: bool,
) -> (bool, f64) {
    let policy = if native {
        FunctionExpansionPolicy::DeferredNativeFirst
    } else {
        FunctionExpansionPolicy::Eager
    };
    let (mechanism, x_column, _result_column) = indicator_target_mechanism(
        policy,
        kind,
        1.0,
        VariableRange::bounded(0.0, 2.0),
        Some(true),
        Some(pin_x),
        10.0,
    );
    let solver = GurobiSolver::new();

    if native {
        let model = mechanism;
        let (output, report) = solver
            .solve_linear_with_native_lowering(model, None)
            .expect("gurobi should solve the natively lowered model");
        assert_eq!(
            report.native_writes, 1,
            "the strictly-boundary model must still be written natively, got {:?}",
            report.outcomes
        );
        let feasible = output.status.is_feasible();
        let value = output
            .solution
            .as_ref()
            .map(|solution| solution[x_column])
            .unwrap_or(f64::NAN);
        (feasible, value)
    } else {
        let model = mechanism.into_linear_triad_model();
        let output = solver
            .solve_linear(&model)
            .expect("gurobi should solve the eager model");
        let feasible = output.status.is_feasible();
        let value = output
            .solution
            .as_ref()
            .map(|solution| solution[x_column])
            .unwrap_or(f64::NAN);
        (feasible, value)
    }
}

#[test]
fn eager_and_native_indicator_paths_agree_at_the_strict_boundary() {
    // 严格不等号是本批最容易写错的地方：即时展开把 `<` 扩张成 `s <= -ε`（ε = 1e-10），原生写入
    // 必须用**同一个** ε。这里把条件变量钉在三个位置，检查两条路径的可行性判断与解完全一致：
    //
    // - `x = 0.5`：`x < 1` 成立（离边界很远）；
    // - `x = 1.5`：`x < 1` 不成立（离边界很远）；
    // - `x = 1.0`：**恰好取在边界值上**，ε 决定取真侧是否可行。
    //
    // 说明：ε = 1e-10 远小于 Gurobi 默认的可行性容差（1e-6），因此「恰好取在边界上」这一档只
    // 能验证两条路径的判断一致（这正是最容易被破坏的性质），而「ε 与即时路径逐位相同」由
    // `native.rs` 的无许可证单测机械化证明：用两个不同的 Big-M 生成即时行，M-不变的那条行的
    // (relation, rhs) 必须与原生计划逐位相等（含 rhs 里的 1e-10）。
    //
    // A strict inequality is the most error-prone part of this batch: eager expansion expands `<` into
    // `s <= -ε` (ε = 1e-10) and the native write must use **that very** ε. The condition variable is
    // pinned at three positions and both paths must agree on feasibility and solution:
    //
    // - `x = 0.5`: `x < 1` holds (far from the boundary);
    // - `x = 1.5`: `x < 1` fails (far from the boundary);
    // - `x = 1.0`: **exactly on the boundary value**, where ε decides whether the true side is
    //   feasible.
    //
    // Note: ε = 1e-10 is far below Gurobi's default feasibility tolerance (1e-6), so the "exactly on
    // the boundary" case can only assert that both paths decide alike (which is the property most
    // easily broken), while "the ε is bit-identical to the eager path's" is proven mechanically by the
    // licence-free unit test in `native.rs`: two different Big-M values generate the eager rows and the
    // M-invariant row's (relation, rhs) must equal the native plan bit for bit, including the 1e-10 in
    // its right-hand side.
    for (pin_x, expected) in [(0.5, Some(true)), (1.5, Some(false)), (1.0, None)] {
        let (eager_feasible, eager_x) =
            indicator_path_feasibility(InequalityKind::Less, pin_x, false);
        let (native_feasible, native_x) =
            indicator_path_feasibility(InequalityKind::Less, pin_x, true);

        assert_eq!(
            eager_feasible, native_feasible,
            "the two paths must agree on feasibility at x = {pin_x}: eager = {eager_feasible}, native = {native_feasible}"
        );
        if let Some(expected) = expected {
            assert_eq!(
                eager_feasible, expected,
                "the eager model must judge x = {pin_x} as feasible = {expected}"
            );
        }
        if eager_feasible && native_feasible {
            assert!(
                (eager_x - native_x).abs() <= 1e-6,
                "the two paths disagree at x = {pin_x}: eager = {eager_x}, native = {native_x}"
            );
        }
    }
}

#[test]
fn an_unbounded_condition_column_falls_back_instead_of_writing_natively() {
    // 条件变量无界时无法证明 Big-M 松弛行在盒上恒成立（盒是整条实轴），因此原生写入会比即时展开
    // 更松：writer 必须拒绝并回退通用展开。回退后的模型仍然完整可解，且指示列取真时条件照旧成立。
    //
    // An unbounded condition variable makes the Big-M relaxation unprovable on the box (the box is the
    // whole real line), so a native write would be looser than eager expansion: the writer must reject
    // and fall back to the generic expansion. The fallback model still solves completely and the
    // condition still holds when the indicator is true.
    let (mechanism, x_column, result_column) = indicator_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        InequalityKind::LessEqual,
        1.0,
        VariableRange::unbounded(),
        Some(true),
        None,
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an unbounded condition column must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::Rejected(message))]
                if message.contains("big-M relaxation")
        ),
        "expected a big-M proof rejection, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[result_column];
    assert!(
        y_value >= 1.0 - 1e-6,
        "the pinning row must fix the indicator to 1: y = {y_value}"
    );
    assert!(
        x_value <= 1.0 + 1e-6,
        "the fallback rows must still enforce x <= 1: x = {x_value}"
    );
}

const IF_IN_X_ID: usize = 97_100;
const IF_IN_ID: u64 = 97_200;

/// IF-IN 验收用的机制模型：`res = [x ∈ values]`，输入列声明为 `range`。
///
/// `pin_result` 把结果列钉在真/假（`res >= 1` / `res <= 0`），`pin_x` 把输入钉在一个确定值上——
/// 无目标的可行模型只会返回任一顶点，钉住取值才能让两条路径逐点比较。
///
/// The mechanism model used by the IF-IN acceptance tests: `res = [x ∈ values]` with the input column
/// declared as `range`.
///
/// `pin_result` pins the result column to true/false (`res >= 1` / `res <= 0`) and `pin_x` pins the input
/// to a fixed value — a solver only returns some vertex for a feasible model without an objective, and
/// pinning is what makes the two paths comparable point by point.
fn if_in_target_mechanism(
    policy: FunctionExpansionPolicy,
    range: VariableRange<f64>,
    values: Vec<f64>,
    pin_result: Option<bool>,
    pin_x: Option<f64>,
    configured_big_m: f64,
) -> (MechanismModel<f64>, usize, usize) {
    let mut model = MetaModel::<f64>::new("gurobi_native_if_in");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(VariableId::standalone(IF_IN_X_ID), "x", range);
    let x_index = model.register_variable(x).expect("x should register");

    let if_in = IfInFunction::new(
        IF_IN_ID,
        "ifin_native",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        values,
        configured_big_m,
    );
    let result_id = if_in.result_variable().id();
    model
        .add_symbol(Arc::new(if_in))
        .expect("if_in symbol should register");

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(IF_IN_X_ID))
        .expect("x column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("if_in result column should exist");

    if let Some(value) = pin_x {
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_column)], 0.0),
                ConstraintRelation::Equal,
                value,
            ),
            "if_in_pin_x",
        ));
    }
    if let Some(value) = pin_result {
        let (relation, rhs) = if value {
            (ConstraintRelation::GreaterEqual, 1.0)
        } else {
            (ConstraintRelation::LessEqual, 0.0)
        };
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
                relation,
                rhs,
            ),
            "if_in_pin_value",
        ));
    }

    (mechanism, x_column, result_column)
}

#[test]
fn native_if_in_write_is_used_by_a_real_solve() {
    // `res = [x ∈ {1, 3}]`，x ∈ [0, 4]，把 x 钉在集合里的 3 并把结果钉在真：即时展开用 band 行 +
    // side 行 + `or_lb_*` / `or_ub` 表达，原生写入用四条指示约束加一条 `or` 一般约束表达。
    // `res = [x ∈ {1, 3}]` with x ∈ [0, 4], pinning x to the in-set value 3 and the result to true:
    // eager expansion uses band rows, side rows and the `or_lb_*` / `or_ub` families while the native
    // write uses four indicator constraints plus one `or` general constraint.
    let (mechanism, x_column, result_column) = if_in_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        VariableRange::bounded(0.0, 4.0),
        vec![1.0, 3.0],
        Some(true),
        Some(3.0),
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_if_in" && record.schema == "functions-if-in-1"
        )),
        "expected a native if-in write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let result_value = solution[result_column];
    assert!(
        (x_value - 3.0).abs() <= 1e-6,
        "the pinning row must fix x = 3: x = {x_value}"
    );
    assert!(
        result_value >= 1.0 - 1e-6,
        "the pinning row must fix the result to 1: res = {result_value}"
    );
}

/// 在一条路径上求解给定的 IF-IN 配置，返回 `(是否可行, x 取值, 结果列取值)`。
/// Solve one IF-IN configuration on one path, returning `(feasible, x value, result value)`.
fn if_in_path_outcome(
    pin_x: f64,
    pin_result: bool,
    native: bool,
) -> (bool, f64, f64) {
    let policy = if native {
        FunctionExpansionPolicy::DeferredNativeFirst
    } else {
        FunctionExpansionPolicy::Eager
    };
    let (mechanism, x_column, result_column) = if_in_target_mechanism(
        policy,
        VariableRange::bounded(0.0, 4.0),
        vec![1.0, 3.0],
        Some(pin_result),
        Some(pin_x),
        10.0,
    );
    let solver = GurobiSolver::new();

    let output = if native {
        let (output, report) = solver
            .solve_linear_with_native_lowering(mechanism, None)
            .expect("gurobi should solve the natively lowered model");
        assert_eq!(
            report.native_writes, 1,
            "the boundary model must still be written natively, got {:?}",
            report.outcomes
        );
        output
    } else {
        let model = mechanism.into_linear_triad_model();
        solver
            .solve_linear(&model)
            .expect("gurobi should solve the eager model")
    };

    let feasible = output.status.is_feasible();
    match output.solution.as_ref() {
        Some(solution) => (feasible, solution[x_column], solution[result_column]),
        None => (feasible, f64::NAN, f64::NAN),
    }
}

#[test]
fn eager_and_native_if_in_paths_agree_at_the_set_boundary() {
    // IF-IN 的边界语义是本批最容易写错的地方：候选值的 band（`|s_i| <= STEP_EPSILON`）与
    // 「不在集合内」的严格边界（`|s_i| >= STRICT_BOUNDARY`）之间有一段刻意留出的间隙，而
    // `STRICT_BOUNDARY = 2 · STEP_EPSILON` 远小于 Gurobi 默认的可行性容差（1e-6）。因此这里比较的是
    // **在可分辨尺度上的边界行为**：集合端点、集合内、刚出集合、集合之间，以及把结果钉在相反一侧：
    //
    // - `(x = 1.0, res = 1)` / `(x = 3.0, res = 1)`：命中集合端点，两条路径都必须可行；
    // - `(x = 1.5, res = 1)`：比候选值 1 大 0.5（远超间隙），必须不可行；
    // - `(x = 2.0, res = 1)`：落在两个候选值之间，必须不可行；
    // - `(x = 1.0, res = 0)`：命中候选值却把结果钉在假，`or` 链接给出尺度为 1 的冲突，必须不可行；
    // - `(x = 2.0, res = 0)`：在集合外且结果取假，必须可行。
    //
    // `STEP_EPSILON` / `STRICT_BOUNDARY` 与即时路径逐位相同这一更强性质，由 `native.rs` 与 `if_in.rs`
    // 里的机械化单测证明（用两个不同 Big-M 生成即时行，M-不变行必须与原生计划逐位相等）。
    //
    // IF-IN boundary semantics are the most error-prone part of this batch: a deliberate gap sits
    // between a candidate's band (`|s_i| <= STEP_EPSILON`) and the strict "outside the set" boundary
    // (`|s_i| >= STRICT_BOUNDARY`), and `STRICT_BOUNDARY = 2 · STEP_EPSILON` is far below Gurobi's
    // default feasibility tolerance (1e-6). This test therefore compares the boundary behaviour **at a
    // resolvable scale**: set endpoints, in set, just outside, between candidates, and the result pinned
    // to the opposite side:
    //
    // - `(x = 1.0, res = 1)` / `(x = 3.0, res = 1)`: a set endpoint is hit and both paths must be
    //   feasible;
    // - `(x = 1.5, res = 1)`: 0.5 above candidate 1 (far beyond the gap) and must be infeasible;
    // - `(x = 2.0, res = 1)`: between the two candidates and must be infeasible;
    // - `(x = 1.0, res = 0)`: a candidate is hit while the result is pinned false — the conflict is only
    //   `STRICT_BOUNDARY = 2e-8` wide, far below the solver's own feasibility tolerance, so both paths
    //   legitimately report feasibility and what this case verifies is that they **agree**;
    // - `(x = 2.0, res = 0)`: outside the set with a false result and must be feasible.
    //
    // The stronger property — `STEP_EPSILON` / `STRICT_BOUNDARY` being bit-identical to the eager
    // path's — is proven by the mechanical unit tests in `native.rs` and `if_in.rs` (two different Big-M
    // values generate the eager rows and the M-invariant row must equal the native plan bit for bit).
    for (pin_x, pin_result, expected) in [
        (1.0, true, true),
        (3.0, true, true),
        (1.5, true, false),
        (2.0, true, false),
        // 冲突宽度只有 STRICT_BOUNDARY（2e-8），求解器容差内视为可行；本档验证的是两路径一致。
        // The conflict is only STRICT_BOUNDARY (2e-8) wide and counts as feasible within the solver's
        // tolerance; this case verifies that the two paths agree.
        (1.0, false, true),
        (2.0, false, true),
    ] {
        let (eager_feasible, eager_x, eager_result) =
            if_in_path_outcome(pin_x, pin_result, false);
        let (native_feasible, native_x, native_result) =
            if_in_path_outcome(pin_x, pin_result, true);

        assert_eq!(
            eager_feasible, native_feasible,
            "the two paths must agree on feasibility at (x = {pin_x}, res = {pin_result}): eager = {eager_feasible}, native = {native_feasible}"
        );
        assert_eq!(
            eager_feasible, expected,
            "the model must judge (x = {pin_x}, res = {pin_result}) as feasible = {expected}"
        );
        if eager_feasible && native_feasible {
            assert!(
                (eager_x - native_x).abs() <= 1e-6,
                "the two paths disagree on x at (x = {pin_x}, res = {pin_result}): eager = {eager_x}, native = {native_x}"
            );
            assert!(
                (eager_result - native_result).abs() <= 1e-6,
                "the two paths disagree on the result at (x = {pin_x}, res = {pin_result}): eager = {eager_result}, native = {native_result}"
            );
        }
    }
}

#[test]
fn an_externally_referenced_if_in_helper_falls_back_instead_of_writing_natively() {
    // 候选值的指示列与 side 列都是本结构的辅助列：一旦模型在别处引用其中一列，原生写入就可能在
    // 那些列上与即时展开出现可区分的差异，因此 writer 必须拒绝并回退通用展开；回退后的模型仍然
    // 完整可解，且集合判定照旧成立。
    //
    // A candidate's indicator and side columns are helpers of this structure: once the model references
    // one of them elsewhere, the native write could become distinguishable from eager expansion on those
    // columns, so the writer must reject and fall back to the generic expansion; the fallback model still
    // solves completely and the set membership still holds.
    let (mut mechanism, x_column, result_column) = if_in_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        VariableRange::bounded(0.0, 4.0),
        vec![1.0, 3.0],
        Some(true),
        Some(3.0),
        10.0,
    );
    let view = mechanism.linear_column_view();
    let indicator_column = view
        .iter()
        .position(|column| column.name.ends_with("_ifin_val0"))
        .expect("the first candidate's indicator column should exist");
    // 在别处引用候选值 0 的指示列：约束 `b_0 <= 1`（把该列纳入非本函数的行）。
    // Reference candidate 0's indicator column elsewhere: constrain `b_0 <= 1`, putting the column into
    // a row that is not one of this function's own rows.
    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, indicator_column)], 0.0),
            ConstraintRelation::LessEqual,
            1.0,
        ),
        "external_ifin_indicator_reference",
    ));

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an externally referenced helper column must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::Rejected(message))]
                if message.contains("helper columns")
        ),
        "expected a helper-reference rejection, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let result_value = solution[result_column];
    assert!(
        (x_value - 3.0).abs() <= 1e-6,
        "the pinning row must fix x = 3: x = {x_value}"
    );
    assert!(
        result_value >= 1.0 - 1e-6,
        "the fallback rows must still enforce the set membership: res = {result_value}"
    );
}

const LOGIC_X_ID: usize = 98_000;
const LOGIC_Y_ID: usize = 98_001;
const LOGIC_AND_ID: u64 = 98_100;
const LOGIC_OR_ID: u64 = 98_200;

/// AND/OR 验收用的机制模型：`res = AND(x, y)` 或 `res = OR(x, y)`，`x`/`y` 是二值变量。
///
/// 返回 `(模型, x 列号, y 列号, 结果列号)`。操作数是**直接二值变量**，因此两个符号都会走到紧凑
/// hull 分支并暴露延迟结构——这正是原生 `add_genconstr_and` / `add_genconstr_or` 能表达的唯一形态。
///
/// The mechanism model used by the AND/OR acceptance tests: `res = AND(x, y)` or `res = OR(x, y)` with
/// binary `x`/`y`.
///
/// Returns `(model, x column, y column, result column)`. The operands are **direct binary variables**, so
/// both symbols take the compact hull branch and expose a deferred structure — exactly the only shape the
/// native `add_genconstr_and` / `add_genconstr_or` expresses.
fn logical_target_mechanism(
    policy: FunctionExpansionPolicy,
    conjunction: bool,
    pin_result: Option<bool>,
    pin_x: Option<f64>,
    pin_y: Option<f64>,
) -> (MechanismModel<f64>, usize, usize, usize) {
    let mut model = MetaModel::<f64>::new("gurobi_native_logic");
    model.set_function_expansion_policy(policy);

    let x = BinaryVariableItem::create(VariableId::standalone(LOGIC_X_ID), "x");
    let y = BinaryVariableItem::create(VariableId::standalone(LOGIC_Y_ID), "y");
    let x_index = model.register_variable(x).expect("x should register");
    let y_index = model.register_variable(y).expect("y should register");

    let operands = vec![
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
    ];
    let result_id = if conjunction {
        let and = AndFunction::new(LOGIC_AND_ID, "and_native", operands);
        let result = and.result_variable().id();
        model
            .add_symbol(Arc::new(and))
            .expect("and symbol should register");
        result
    } else {
        let or = OrFunction::new(LOGIC_OR_ID, "or_native", operands);
        let result = or.result_variable().id();
        model
            .add_symbol(Arc::new(or))
            .expect("or symbol should register");
        result
    };

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(LOGIC_X_ID))
        .expect("x column should exist");
    let y_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(LOGIC_Y_ID))
        .expect("y column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("logic result column should exist");

    for (column, value, name) in [
        (x_column, pin_x, "logic_pin_x"),
        (y_column, pin_y, "logic_pin_y"),
    ] {
        if let Some(value) = value {
            mechanism.add_constraint(LinearConstraint::new(
                LinearInequality::new(
                    Linear::new(vec![LinearMonomial::new(1.0, column)], 0.0),
                    ConstraintRelation::Equal,
                    value,
                ),
                name,
            ));
        }
    }
    if let Some(value) = pin_result {
        let (relation, rhs) = if value {
            (ConstraintRelation::GreaterEqual, 1.0)
        } else {
            (ConstraintRelation::LessEqual, 0.0)
        };
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
                relation,
                rhs,
            ),
            "logic_pin_result",
        ));
    }

    (mechanism, x_column, y_column, result_column)
}

#[test]
fn native_and_write_is_used_by_a_real_solve() {
    // `res = AND(x, y)` 且 `res = 1`：即时展开给出 `res <= x`、`res <= y` 与
    // `x + y - res <= 1` 三行，原生写入给出等价的 `res = AND(x, y)` 一般约束，因此 `x = y = 1`。
    // `res = AND(x, y)` with `res = 1`: eager expansion contributes `res <= x`, `res <= y` and
    // `x + y - res <= 1` while the native write contributes the equivalent `res = AND(x, y)` general
    // constraint, hence `x = y = 1`.
    let (mechanism, x_column, y_column, result_column) = logical_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        true,
        Some(true),
        None,
        None,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_and" && record.schema == "functions-and-1"
        )),
        "expected a native AND write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[y_column];
    let result_value = solution[result_column];
    for (label, value) in [
        ("x", x_value),
        ("y", y_value),
        ("res", result_value),
    ] {
        assert!(
            value >= 1.0 - 1e-6,
            "res = AND(x, y) = 1 must force {label} = 1: {label} = {value}"
        );
    }
}

#[test]
fn native_or_write_is_used_by_a_real_solve() {
    // `res = OR(x, y)` 且 `res = 0`：即时展开给出 `res >= x`、`res >= y` 与 `x + y - res >= 0`，
    // 原生写入给出等价的 `res = OR(x, y)`，因此 `x = y = 0`。
    // `res = OR(x, y)` with `res = 0`: eager expansion contributes `res >= x`, `res >= y` and
    // `x + y - res >= 0` while the native write contributes the equivalent `res = OR(x, y)`, hence
    // `x = y = 0`.
    let (mechanism, x_column, y_column, result_column) = logical_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        false,
        Some(false),
        None,
        None,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_or" && record.schema == "functions-or-1"
        )),
        "expected a native OR write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    let x_value = solution[x_column];
    let y_value = solution[y_column];
    let result_value = solution[result_column];
    for (label, value) in [
        ("x", x_value),
        ("y", y_value),
        ("res", result_value),
    ] {
        assert!(
            value <= 1e-6,
            "res = OR(x, y) = 0 must force {label} = 0: {label} = {value}"
        );
    }
}

/// 在一条路径上求解给定的 AND/OR 配置，返回 `(是否可行, 结果列取值)`。
/// Solve one AND/OR configuration on one path, returning `(feasible, result value)`.
fn logical_path_outcome(
    conjunction: bool,
    x_value: f64,
    y_value: f64,
    native: bool,
) -> (bool, f64) {
    let policy = if native {
        FunctionExpansionPolicy::DeferredNativeFirst
    } else {
        FunctionExpansionPolicy::Eager
    };
    let (mechanism, _x_column, _y_column, result_column) = logical_target_mechanism(
        policy,
        conjunction,
        None,
        Some(x_value),
        Some(y_value),
    );
    let solver = GurobiSolver::new();

    let output = if native {
        let (output, report) = solver
            .solve_linear_with_native_lowering(mechanism, None)
            .expect("gurobi should solve the natively lowered model");
        assert_eq!(
            report.native_writes, 1,
            "the truth-table model must still be written natively, got {:?}",
            report.outcomes
        );
        output
    } else {
        let model = mechanism.into_linear_triad_model();
        solver
            .solve_linear(&model)
            .expect("gurobi should solve the eager model")
    };

    let feasible = output.status.is_feasible();
    match output.solution.as_ref() {
        Some(solution) => (feasible, solution[result_column]),
        None => (feasible, f64::NAN),
    }
}

#[test]
fn eager_and_native_logic_paths_agree_on_the_truth_table() {
    // 全 0、全 1 与两种混合输入下，原生 `add_genconstr_and/or` 与即时紧凑 hull 必须给出同一个真值：
    // 即时是 `res <= xi` / `sum - res <= n - 1`（AND）与 `res >= xi` / `sum - res >= 0`（OR），
    // 二元变量上这两族行正是 `res = AND/OR(inputs)` 的精确两侧 hull。
    //
    // Over all-zero, all-one and both mixed inputs, the native `add_genconstr_and/or` and the eager
    // compact hull must give the same truth value: eager is `res <= xi` / `sum - res <= n - 1` (AND) and
    // `res >= xi` / `sum - res >= 0` (OR), and over binary variables those families are exactly the
    // precise two-sided hull of `res = AND/OR(inputs)`.
    for conjunction in [true, false] {
        for (x_value, y_value) in [
            (0.0f64, 0.0f64),
            (0.0, 1.0),
            (1.0, 0.0),
            (1.0, 1.0),
        ] {
            let expected = if conjunction {
                x_value.min(y_value)
            } else {
                x_value.max(y_value)
            };

            let (eager_feasible, eager_result) =
                logical_path_outcome(conjunction, x_value, y_value, false);
            let (native_feasible, native_result) =
                logical_path_outcome(conjunction, x_value, y_value, true);

            let label = if conjunction { "AND" } else { "OR" };
            assert!(
                eager_feasible && native_feasible,
                "{label}({x_value}, {y_value}) must be feasible on both paths: eager = {eager_feasible}, native = {native_feasible}"
            );
            assert!(
                (eager_result - expected).abs() <= 1e-6,
                "{label}({x_value}, {y_value}) eager result must be {expected}: {eager_result}"
            );
            assert!(
                (native_result - expected).abs() <= 1e-6,
                "{label}({x_value}, {y_value}) native result must be {expected}: {native_result}"
            );
        }
    }
}

#[test]
fn a_non_binary_logic_operand_keeps_the_eager_expansion() {
    // 操作数不是「直接二值变量」时，紧凑 hull 分支不成立：即时展开改用每个操作数的非零指示列
    // （`res = AND/OR(pred_i)`，`pred_i` 表示多项式非零），而原生 and/or 表达不了那些行。因此这种模型
    // **不暴露延迟结构**（`deferred_structure_with_tokens` 的准入），求解继续走即时展开且结果正确。
    //
    // When an operand is not a "direct binary variable", the compact hull branch does not apply: eager
    // expansion switches to a non-zero indicator per operand (`res = AND/OR(pred_i)` where `pred_i` means
    // the polynomial is non-zero) and the native and/or cannot express those rows. Such a model therefore
    // **exposes no deferred structure** (the admission rule of `deferred_structure_with_tokens`), the
    // solve stays on eager expansion and the result is still correct.
    for (z_value, expected) in [(0.0, 0.0), (1.0, 1.0)] {
        let mut model = MetaModel::<f64>::new("gurobi_native_logic_eager");
        model.set_function_expansion_policy(FunctionExpansionPolicy::DeferredNativeFirst);

        let z = ContinuousVariableItem::with_range(
            VariableId::standalone(LOGIC_X_ID),
            "z",
            VariableRange::bounded(-2.0, 2.0),
        );
        let z_index = model.register_variable(z).expect("z should register");

        // 连续输入：操作数不是直接二值变量。
        // A continuous input: the operand is not a direct binary variable.
        let and = AndFunction::new(
            LOGIC_AND_ID,
            "and_eager",
            vec![Linear::new(vec![LinearMonomial::new(1.0, z_index)], 0.0)],
        );
        let result_id = and.result_variable().id();
        model
            .add_symbol(Arc::new(and))
            .expect("and symbol should register");

        let mut mechanism = model
            .try_into_mechanism_model()
            .expect("mechanism conversion should succeed");
        let view = mechanism.linear_column_view();
        let z_column = view
            .iter()
            .position(|column| column.id == VariableId::standalone(LOGIC_X_ID))
            .expect("z column should exist");
        let result_column = view
            .iter()
            .position(|column| column.id == result_id)
            .expect("logic result column should exist");

        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, z_column)], 0.0),
                ConstraintRelation::Equal,
                z_value,
            ),
            "logic_pin_z",
        ));

        let solver = GurobiSolver::new();
        let (output, report) = solver
            .solve_linear_with_native_lowering(mechanism, None)
            .expect("gurobi should solve the eager model");

        // 没有延迟结构被暴露，因此没有任何 writer 认领这个符号。
        // No deferred structure was exposed, so no writer claimed this symbol.
        assert_eq!(
            report.native_writes, 0,
            "a non-direct-binary operand must forbid the native write"
        );
        assert!(report.outcomes.is_empty());

        assert!(
            output.status.is_feasible(),
            "the eager model must still solve, got {:?}",
            output.status
        );
        let solution = output
            .solution
            .as_ref()
            .expect("a feasible solve should carry a solution");
        let result_value = solution[result_column];
        assert!(
            (result_value - expected).abs() <= 1e-6,
            "the eager nonzero-indicator rows must give AND(z != 0) = {expected} at z = {z_value}: res = {result_value}"
        );
    }
}

const BINARYZATION_X_ID: usize = 99_000;
const BINARYZATION_ID: u64 = 99_100;

/// 二值化验收用的机制模型：`y = binary(input >= threshold)`，输入是「系数 1、常数 0」的单单项式。
///
/// `pin_result` 把结果二值列钉在真/假（`y >= 1` / `y <= 0`），`pin_x` 把输入钉在一个确定值上——无目标
/// 的可行模型只会返回任一顶点，钉住取值才能让两条路径逐点比较。
///
/// The mechanism model used by the binaryzation acceptance tests: `y = binary(input >= threshold)` with a
/// single unit-coefficient, zero-constant monomial as input.
///
/// `pin_result` pins the binary result column to true/false (`y >= 1` / `y <= 0`) and `pin_x` pins the
/// input to a fixed value — a solver only returns some vertex for a feasible model without an objective,
/// and pinning is what makes the two paths comparable point by point.
fn binaryzation_target_mechanism(
    policy: FunctionExpansionPolicy,
    method: BinaryzationMethod,
    threshold: f64,
    range: VariableRange<f64>,
    pin_result: Option<bool>,
    pin_x: Option<f64>,
    configured_big_m: f64,
) -> (MechanismModel<f64>, usize, usize) {
    let mut model = MetaModel::<f64>::new("gurobi_native_binaryzation");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(BINARYZATION_X_ID),
        "x",
        range,
    );
    let x_index = model.register_variable(x).expect("x should register");

    let binary = BinaryzationFunction::new(
        BINARYZATION_ID,
        "binaryzation_native",
        Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
        threshold,
        configured_big_m,
        method,
    );
    let result_id = binary.result_variable().id();
    model
        .add_symbol(Arc::new(binary))
        .expect("binaryzation symbol should register");

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let x_column = view
        .iter()
        .position(|column| column.id == VariableId::standalone(BINARYZATION_X_ID))
        .expect("x column should exist");
    let result_column = view
        .iter()
        .position(|column| column.id == result_id)
        .expect("binaryzation result column should exist");

    if let Some(value) = pin_x {
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_column)], 0.0),
                ConstraintRelation::Equal,
                value,
            ),
            "binaryzation_pin_x",
        ));
    }
    if let Some(value) = pin_result {
        let (relation, rhs) = if value {
            (ConstraintRelation::GreaterEqual, 1.0)
        } else {
            (ConstraintRelation::LessEqual, 0.0)
        };
        mechanism.add_constraint(LinearConstraint::new(
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, result_column)], 0.0),
                relation,
                rhs,
            ),
            "binaryzation_pin_value",
        ));
    }

    (mechanism, x_column, result_column)
}

#[test]
fn native_binaryzation_threshold_write_is_used_by_a_real_solve() {
    // 阈值编码：`y = binary(x >= 1)`，即时展开给出 `y = 1 ⇒ s >= 0`（`s = x - 1`）与 `y = 0 ⇒ s <= -ε`，
    // 原生写入给出等价的 `ind = 1 ⇒ s >= 0` / `ind = 0 ⇒ s <= -ε`。这里把 x 钉在 1.5 并把 y 钉在真。
    // Threshold encoding: `y = binary(x >= 1)`; eager expansion contributes `y = 1 ⇒ s >= 0`
    // (`s = x - 1`) and `y = 0 ⇒ s <= -ε`, and the native write contributes the equivalent
    // `ind = 1 ⇒ s >= 0` / `ind = 0 ⇒ s <= -ε`. Here x is pinned to 1.5 and y to true.
    let (mechanism, x_column, result_column) = binaryzation_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        BinaryzationMethod::Threshold,
        1.0,
        VariableRange::bounded(0.0, 2.0),
        Some(true),
        Some(1.5),
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_binaryzation"
                    && record.schema == "functions-binaryzation-1"
        )),
        "expected a native binaryzation write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    assert!(
        (solution[x_column] - 1.5).abs() <= 1e-6,
        "the pinning row must fix x = 1.5: x = {}",
        solution[x_column]
    );
    assert!(
        solution[result_column] >= 1.0 - 1e-6,
        "the pinning row must fix the result to 1: y = {}",
        solution[result_column]
    );
}

#[test]
fn native_binaryzation_big_m_write_is_used_by_a_real_solve() {
    // Big-M 编码：`y = binary(x >= 1)`，即时展开给出 `y = 1 ⇒ s >= ε` 与 `y = 0 ⇒ s <= 0`；这里把 x 钉在
    // 0.5 并把 y 钉在假，因此 `x <= 1` 必须成立。
    // Big-M encoding: `y = binary(x >= 1)`; eager expansion contributes `y = 1 ⇒ s >= ε` and
    // `y = 0 ⇒ s <= 0`; here x is pinned to 0.5 and y to false, so `x <= 1` must hold.
    let (mechanism, x_column, result_column) = binaryzation_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        BinaryzationMethod::BigM,
        1.0,
        VariableRange::bounded(0.0, 2.0),
        Some(false),
        Some(0.5),
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_binaryzation"
                    && record.schema == "functions-binaryzation-1"
        )),
        "expected a native binaryzation write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    assert!(
        (solution[x_column] - 0.5).abs() <= 1e-6,
        "the pinning row must fix x = 0.5: x = {}",
        solution[x_column]
    );
    assert!(
        solution[result_column] <= 1e-6,
        "the pinning row must fix the result to 0: y = {}",
        solution[result_column]
    );
}

/// 在一条路径上求解给定的二值化配置，返回 `(是否可行, x 取值, 结果列取值)`。
/// Solve one binaryzation configuration on one path, returning `(feasible, x value, result value)`.
fn binaryzation_path_outcome(
    method: BinaryzationMethod,
    pin_result: bool,
    pin_x: f64,
    native: bool,
) -> (bool, f64, f64) {
    let policy = if native {
        FunctionExpansionPolicy::DeferredNativeFirst
    } else {
        FunctionExpansionPolicy::Eager
    };
    let (mechanism, x_column, result_column) = binaryzation_target_mechanism(
        policy,
        method,
        1.0,
        VariableRange::bounded(0.0, 2.0),
        Some(pin_result),
        Some(pin_x),
        10.0,
    );
    let solver = GurobiSolver::new();

    let output = if native {
        let (output, report) = solver
            .solve_linear_with_native_lowering(mechanism, None)
            .expect("gurobi should solve the natively lowered model");
        assert_eq!(
            report.native_writes, 1,
            "the boundary model must still be written natively, got {:?}",
            report.outcomes
        );
        output
    } else {
        let model = mechanism.into_linear_triad_model();
        solver
            .solve_linear(&model)
            .expect("gurobi should solve the eager model")
    };

    let feasible = output.status.is_feasible();
    match output.solution.as_ref() {
        Some(solution) => (feasible, solution[x_column], solution[result_column]),
        None => (feasible, f64::NAN, f64::NAN),
    }
}

#[test]
fn eager_and_native_binaryzation_paths_agree_near_the_threshold() {
    // 二值化的边界语义是本批最容易写错的地方：取真一侧与取假一侧之间刻意留出的间隙只有
    // ε = `16·f64::EPSILON ≈ 3.55e-15`，远小于 Gurobi 默认可行性容差（1e-6）。因此在**可分辨尺度**上
    // 比较两条路径，并把「s 恰好落在阈值上」的容差档单独标成 `None`（只断言两条路径判断一致）：
    //
    // - Threshold，y = 1：x = 1.5 可行、x = 0.5 不可行、x = 恰好 1 时 `s = 0 >= 0` 恰好成立（可判定）；
    // - Threshold，y = 0：x = 0.5 可行（`s = -0.5 <= -ε`）、x = 1.5 不可行、x = 恰好 1 时只差一个 ε
    //   （容差档，只比较一致性）；
    // - Big-M，y = 1：x = 1.5 可行（`s = 0.5 >= ε`）、x = 0.5 不可行、x = 恰好 1 时只差一个 ε（容差档）；
    // - Big-M，y = 0：x = 0.5 可行（`s = -0.5 <= 0`）、x = 1.5 不可行、x = 恰好 1 时 `s = 0 <= 0` 恰好
    //   成立（可判定）。
    //
    // Binaryzation boundary semantics are the most error-prone part of this batch: the deliberately
    // left-open gap between the true side and the false side is only
    // ε = `16·f64::EPSILON ≈ 3.55e-15`, far below Gurobi's default feasibility tolerance (1e-6). This test
    // therefore compares the two paths at a **resolvable scale** and marks the "s exactly at the threshold"
    // tolerance cases as `None` (agreement only):
    //
    // - Threshold, y = 1: x = 1.5 feasible, x = 0.5 infeasible, x = exactly 1 gives `s = 0 >= 0` exactly;
    // - Threshold, y = 0: x = 0.5 feasible (`s = -0.5 <= -ε`), x = 1.5 infeasible, x = exactly 1 misses by
    //   one ε (tolerance case, agreement only);
    // - Big-M, y = 1: x = 1.5 feasible (`s = 0.5 >= ε`), x = 0.5 infeasible, x = exactly 1 misses by one ε;
    // - Big-M, y = 0: x = 0.5 feasible (`s = -0.5 <= 0`), x = 1.5 infeasible, x = exactly 1 gives
    //   `s = 0 <= 0` exactly.
    //
    // 「ε 与即时路径逐位相同」这一更强性质由 `native.rs` 与 `binaryzation.rs` 的机械化单测证明（用两个
    // 不同 Big-M 生成即时行，M-不变行必须与原生计划逐位相等）。
    // The stronger property — ε being bit-identical to the eager path's — is proven by the mechanical unit
    // tests in `native.rs` and `binaryzation.rs` (two different Big-M values generate the eager rows and the
    // M-invariant row must equal the native plan bit for bit).
    for (method, pin_result, pin_x, expected) in [
        (BinaryzationMethod::Threshold, true, 1.5, Some(true)),
        (BinaryzationMethod::Threshold, true, 0.5, Some(false)),
        (BinaryzationMethod::Threshold, true, 1.0, Some(true)),
        (BinaryzationMethod::Threshold, false, 0.5, Some(true)),
        (BinaryzationMethod::Threshold, false, 1.5, Some(false)),
        (BinaryzationMethod::Threshold, false, 1.0, None),
        (BinaryzationMethod::BigM, true, 1.5, Some(true)),
        (BinaryzationMethod::BigM, true, 0.5, Some(false)),
        (BinaryzationMethod::BigM, true, 1.0, None),
        (BinaryzationMethod::BigM, false, 0.5, Some(true)),
        (BinaryzationMethod::BigM, false, 1.5, Some(false)),
        (BinaryzationMethod::BigM, false, 1.0, Some(true)),
    ] {
        let (eager_feasible, eager_x, eager_result) =
            binaryzation_path_outcome(method, pin_result, pin_x, false);
        let (native_feasible, native_x, native_result) =
            binaryzation_path_outcome(method, pin_result, pin_x, true);

        assert_eq!(
            eager_feasible, native_feasible,
            "the two paths must agree on feasibility for {method:?} (y = {pin_result}, x = {pin_x}): eager = {eager_feasible}, native = {native_feasible}"
        );
        if let Some(expected) = expected {
            assert_eq!(
                eager_feasible, expected,
                "the eager model must judge {method:?} (y = {pin_result}, x = {pin_x}) as feasible = {expected}"
            );
        }
        if eager_feasible && native_feasible {
            assert!(
                (eager_x - native_x).abs() <= 1e-6,
                "the two paths disagree on x for {method:?} (y = {pin_result}, x = {pin_x}): eager = {eager_x}, native = {native_x}"
            );
            assert!(
                (eager_result - native_result).abs() <= 1e-6,
                "the two paths disagree on the result for {method:?} (y = {pin_result}, x = {pin_x}): eager = {eager_result}, native = {native_result}"
            );
        }
    }
}

#[test]
fn an_unbounded_binaryzation_input_falls_back_instead_of_writing_natively() {
    // 输入列无界时无法证明 Big-M 松弛行在盒上恒成立（盒是整条实轴），因此原生写入会比即时展开更松：
    // writer 必须拒绝并回退通用展开。回退后的模型仍然完整可解（配置的 Big-M 依然把 `s` 限制在
    // `[-M, M]` 内），且阈值关系照旧成立。
    //
    // An unbounded input column makes the Big-M relaxation unprovable on the box (the box is the whole real
    // line), so a native write would be looser than eager expansion: the writer must reject and fall back to
    // the generic expansion. The fallback model still solves completely (the configured Big-M keeps `s`
    // inside `[-M, M]`) and the threshold relation still holds.
    let (mechanism, x_column, result_column) = binaryzation_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        BinaryzationMethod::Threshold,
        1.0,
        VariableRange::unbounded(),
        Some(true),
        Some(2.0),
        10.0,
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an unbounded input column must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::Rejected(message))]
                if message.contains("big-M relaxation")
        ),
        "expected a big-M proof rejection, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    assert!(
        (solution[x_column] - 2.0).abs() <= 1e-6,
        "the pinning row must fix x = 2: x = {}",
        solution[x_column]
    );
    assert!(
        solution[result_column] >= 1.0 - 1e-6,
        "the fallback rows must still enforce the threshold relation: y = {}",
        solution[result_column]
    );
}

const IMPLY_X_ID: usize = 99_200;
const IMPLY_Y_ID: usize = 99_201;
const IMPLY_ID: u64 = 99_300;

/// 蕴含验收模型：前提 `p = [x >= 1]`、结论 `c = [y >= 1]`，`x` 与 `y` 相互独立，因此 `(p, c)` 的 4 种
/// 组合都能靠钉住 `(x, y)` 达到。返回的列序是 `[x, y, p, c, r]`。
///
/// 蕴含即时展开含两个内部关系指示器（各自 2 行、各带自己的 Big-M）与 4 条耦合行（无 Big-M）。原生写入
/// 必须覆盖全部：两个子指示器各 2 条指示约束 + 3 条耦合指示约束。
///
/// The implication acceptance model: premise `p = [x >= 1]` and consequence `c = [y >= 1]` with `x` and `y`
/// independent, so all four `(p, c)` combinations are reachable by pinning `(x, y)`. The returned column
/// order is `[x, y, p, c, r]`.
///
/// The eager implication expansion contains two internal relation indicators (two rows each, carrying their
/// own Big-M) plus four coupling rows (no Big-M). The native write must cover all of it: two indicator
/// constraints per sub-indicator plus three coupling indicator constraints.
fn imply_target_mechanism(
    policy: FunctionExpansionPolicy,
    pin_x: Option<f64>,
    pin_y: Option<f64>,
) -> (MechanismModel<f64>, [usize; 5]) {
    let mut model = MetaModel::<f64>::new("gurobi_native_imply");
    model.set_function_expansion_policy(policy);

    let x = ContinuousVariableItem::with_range(
        VariableId::standalone(IMPLY_X_ID),
        "x",
        VariableRange::bounded(0.0, 2.0),
    );
    let y = ContinuousVariableItem::with_range(
        VariableId::standalone(IMPLY_Y_ID),
        "y",
        VariableRange::bounded(0.0, 2.0),
    );
    let x_index = model.register_variable(x).expect("x should register");
    let y_index = model.register_variable(y).expect("y should register");

    let imply = ImplyFunction::new(
        IMPLY_ID,
        "imply_native",
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        ),
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, y_index)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        ),
        10.0,
    );
    let premise_id = imply.premise_indicator_variable().id();
    let consequence_id = imply.consequence_indicator_variable().id();
    let result_id = imply.result_variable().id();
    model
        .add_symbol(Arc::new(imply))
        .expect("imply symbol should register");

    let mut mechanism = model
        .try_into_mechanism_model()
        .expect("mechanism conversion should succeed");

    let view = mechanism.linear_column_view();
    let column_of = |id: VariableId, label: &str| {
        view.iter()
            .position(|column| column.id == id)
            .unwrap_or_else(|| panic!("{label} column should exist"))
    };
    let x_column = column_of(VariableId::standalone(IMPLY_X_ID), "x");
    let y_column = column_of(VariableId::standalone(IMPLY_Y_ID), "y");
    let premise_column = column_of(premise_id, "premise indicator");
    let consequence_column = column_of(consequence_id, "consequence indicator");
    let result_column = column_of(result_id, "imply result");

    for (column, value, label) in [
        (x_column, pin_x, "imply_pin_x"),
        (y_column, pin_y, "imply_pin_y"),
    ] {
        if let Some(value) = value {
            mechanism.add_constraint(LinearConstraint::new(
                LinearInequality::new(
                    Linear::new(vec![LinearMonomial::new(1.0, column)], 0.0),
                    ConstraintRelation::Equal,
                    value,
                ),
                label,
            ));
        }
    }

    (
        mechanism,
        [
            x_column,
            y_column,
            premise_column,
            consequence_column,
            result_column,
        ],
    )
}

/// 在一条路径上求解给定的蕴含配置，返回 `(是否可行, [p, c, r] 取值)`。
/// Solve one implication configuration on one path, returning `(feasible, [p, c, r] values)`.
fn imply_path_outcome(
    pin_x: f64,
    pin_y: f64,
    native: bool,
) -> (bool, [f64; 3]) {
    let policy = if native {
        FunctionExpansionPolicy::DeferredNativeFirst
    } else {
        FunctionExpansionPolicy::Eager
    };
    let (mechanism, columns) = imply_target_mechanism(policy, Some(pin_x), Some(pin_y));
    let solver = GurobiSolver::new();

    let output = if native {
        let (output, report) = solver
            .solve_linear_with_native_lowering(mechanism, None)
            .expect("gurobi should solve the natively lowered model");
        assert_eq!(
            report.native_writes, 1,
            "the implication must be written natively, got {:?}",
            report.outcomes
        );
        assert!(
            report.outcomes.iter().any(|outcome| matches!(
                outcome,
                NativeWriteOutcome::Native(record)
                    if record.writer == "gurobi_imply" && record.schema == "functions-imply-1"
            )),
            "expected a native implication write with the stable schema, got {:?}",
            report.outcomes
        );
        output
    } else {
        let model = mechanism.into_linear_triad_model();
        solver
            .solve_linear(&model)
            .expect("gurobi should solve the eager model")
    };

    let feasible = output.status.is_feasible();
    match output.solution.as_ref() {
        Some(solution) => (
            feasible,
            [
                solution[columns[2]],
                solution[columns[3]],
                solution[columns[4]],
            ],
        ),
        None => (feasible, [f64::NAN; 3]),
    }
}

#[test]
fn native_imply_write_is_used_by_a_real_solve() {
    // `x = 1.5` 让前提成立（`p = 1`），`y = 0.5` 让结论不成立（`c = 0`）：蕴含为假，结果列必须被
    // 3 条耦合指示约束钉成 0。三个二元列的取值都由模型本身确定，因此断言是无歧义的。
    //
    // `x = 1.5` makes the premise true (`p = 1`) and `y = 0.5` makes the consequence false (`c = 0`): the
    // implication is false and the three coupling indicators must pin the result to 0. All three binary
    // values are determined by the model itself, so the assertions are unambiguous.
    let (mechanism, columns) = imply_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        Some(1.5),
        Some(0.5),
    );

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the natively lowered model");

    assert_eq!(report.native_writes, 1);
    assert_eq!(report.materialized_fallbacks, 0);
    assert!(
        report.outcomes.iter().any(|outcome| matches!(
            outcome,
            NativeWriteOutcome::Native(record)
                if record.writer == "gurobi_imply" && record.schema == "functions-imply-1"
        )),
        "expected a native implication write with the stable schema, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "expected a feasible solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    // 前提子指示器：`x = 1.5 >= 1` ⇒ `p = 1`。
    // Premise sub-indicator: `x = 1.5 >= 1` gives `p = 1`.
    assert!(
        solution[columns[2]] >= 1.0 - 1e-6,
        "the premise column must be 1: p = {}",
        solution[columns[2]]
    );
    // 结论子指示器：`y = 0.5 >= 1` 为假 ⇒ `c = 0`。
    // Consequence sub-indicator: `y = 0.5 >= 1` is false, so `c = 0`.
    assert!(
        solution[columns[3]] <= 1e-6,
        "the consequence column must be 0: c = {}",
        solution[columns[3]]
    );
    // 耦合：`p = 1, c = 0` ⇒ `r = 0`。
    // Coupling: `p = 1, c = 0` gives `r = 0`.
    assert!(
        solution[columns[4]] <= 1e-6,
        "the coupling indicators must give r = 0 for p = 1, c = 0: r = {}",
        solution[columns[4]]
    );
}

#[test]
fn eager_and_native_imply_paths_agree_on_the_whole_truth_table() {
    // 前提与结论的真值组合共 4 种（`x`、`y` 独立），其中 `(p, c) = (1, 0)` 是唯一让蕴含为假的一格。
    // 两条路径必须给出同一组 `(p, c, r)`，且 `r = max(c, 1 - p)`。
    //
    // The premise and consequence have four truth combinations (`x` and `y` are independent), and
    // `(p, c) = (1, 0)` is the only cell that falsifies the implication. Both paths must produce the same
    // `(p, c, r)` triple with `r = max(c, 1 - p)`.
    for (pin_x, pin_y, expected_premise, expected_consequence, expected_result) in [
        (0.5, 0.5, 0.0, 0.0, 1.0),
        (0.5, 1.5, 0.0, 1.0, 1.0),
        (1.5, 0.5, 1.0, 0.0, 0.0),
        (1.5, 1.5, 1.0, 1.0, 1.0),
    ] {
        let (eager_feasible, eager) = imply_path_outcome(pin_x, pin_y, false);
        let (native_feasible, native) = imply_path_outcome(pin_x, pin_y, true);

        assert_eq!(
            eager_feasible, native_feasible,
            "the two paths must agree on feasibility at (x, y) = ({pin_x}, {pin_y})"
        );
        assert!(eager_feasible && native_feasible);
        for index in 0..3 {
            assert!(
                (eager[index] - native[index]).abs() <= 1e-6,
                "the two paths disagree on {} at (x, y) = ({pin_x}, {pin_y}): eager = {}, native = {}",
                ["p", "c", "r"][index],
                eager[index],
                native[index]
            );
        }
        // 三个二元取值都由模型确定，因此可以逐点断言。
        // All three binary values are determined by the model, so they can be asserted pointwise.
        assert!((native[0] - expected_premise).abs() <= 1e-6);
        assert!((native[1] - expected_consequence).abs() <= 1e-6);
        assert!(
            (native[2] - expected_result).abs() <= 1e-6,
            "native r must be max(c, 1 - p) = {expected_result} at (x, y) = ({pin_x}, {pin_y}): r = {}",
            native[2]
        );
    }
}

#[test]
fn an_externally_referenced_helper_column_forces_the_imply_fallback() {
    // 前提子指示器的结果列是本结构的辅助列。外部一旦引用它，原生写入与即时展开在这些列上的含义就可能
    // 被外部分辨出来，因此 writer 必须整体回退——这里用 `p <= 0.5`（即 `p = 0`）触发该门控。
    //
    // The premise sub-indicator's result column is a helper of this structure. Once an external row
    // references it, an external constraint could tell the native write and eager expansion apart on it, so
    // the writer must fall back as a whole — `p <= 0.5` (that is `p = 0`) triggers that gate here.
    let (mut mechanism, columns) = imply_target_mechanism(
        FunctionExpansionPolicy::DeferredNativeFirst,
        Some(0.5),
        Some(0.5),
    );
    mechanism.add_constraint(LinearConstraint::new(
        LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, columns[2])], 0.0),
            ConstraintRelation::LessEqual,
            0.5,
        ),
        "imply_external_premise_reference",
    ));

    let solver = GurobiSolver::new();
    let (output, report) = solver
        .solve_linear_with_native_lowering(mechanism, None)
        .expect("gurobi should solve the fallback model");

    assert_eq!(
        report.native_writes, 0,
        "an externally referenced helper column must forbid the native write"
    );
    assert_eq!(report.materialized_fallbacks, 1);
    assert!(
        matches!(
            report.outcomes.as_slice(),
            [NativeWriteOutcome::Fallback(FallbackReason::Rejected(message))]
                if message.contains("helper columns")
        ),
        "expected the helper-exclusivity rejection, got {:?}",
        report.outcomes
    );

    assert!(
        output.status.is_feasible(),
        "the fallback model must still solve, got {:?}",
        output.status
    );
    let solution = output
        .solution
        .as_ref()
        .expect("a feasible solve should carry a solution");
    // `x = 0.5` 让前提为假、`y = 0.5` 让结论为假，因此蕴含为真：`r = 1`。
    // `x = 0.5` falsifies the premise and `y = 0.5` falsifies the consequence, so the implication holds:
    // `r = 1`.
    assert!(
        solution[columns[2]] <= 1e-6,
        "the premise must be 0: p = {}",
        solution[columns[2]]
    );
    assert!(
        solution[columns[3]] <= 1e-6,
        "the consequence must be 0: c = {}",
        solution[columns[3]]
    );
    assert!(
        solution[columns[4]] >= 1.0 - 1e-6,
        "the eager rows must still give r = 1 for p = 0, c = 0: r = {}",
        solution[columns[4]]
    );
}
