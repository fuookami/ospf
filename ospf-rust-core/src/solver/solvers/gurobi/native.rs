//! Gurobi 原生函数 lowering / Gurobi native function lowering
//!
//! 本模块把求解器无关的延迟结构落到 Gurobi 的 general constraint 上。它只在启用 `gurobi*`
//! feature 时编译，并保持两个层次分离：
//!
//! - **准入判定（不依赖 SDK）**：`plan_abs_native` 是纯函数，决定某个 `AbsStructure` 是否可以
//!   原生写入以及需要哪些列。它可以在没有许可证的机器上被测试，也保证"不接受就回退"的规则不
//!   会被 SDK 调用掩盖。
//! - **SDK 写入**：`GurobiAbsWriter` 通过 [`crate::model::intermediate::NativeFunctionWriter`]
//!   接入统一的 native writer registry，只在准入通过后调用 `add_genconstr_abs`。
//!
//! This module lowers solver-neutral deferred structures onto Gurobi general constraints. It only
//! compiles with a `gurobi*` feature and keeps two layers apart:
//!
//! - **Admission (SDK-free)**: `plan_abs_native` is a pure function deciding whether an
//!   `AbsStructure` may be written natively and which columns it needs. It is testable on a machine
//!   without a licence and keeps the "reject, then fall back" rules from being hidden behind SDK
//!   calls.
//! - **SDK write**: `GurobiAbsWriter` plugs into the unified native writer registry through
//!   [`crate::model::intermediate::NativeFunctionWriter`] and only calls `add_genconstr_abs` after
//!   admission succeeded.

use crate::error::{ModelError, Result};
use crate::model::ConstraintRelation;
use crate::model::flatten::Linear;
use crate::model::intermediate::{
    DeferredFunctionStructure, FallbackReason, NativeFunctionWriter, NativeWriteOutcome,
    NativeWriteRecord, NativeWriteRequest,
};
use crate::symbol::function::{
    AbsStructure, AndStructure, BinaryzationStructure, CosStructure, INDICATOR_TOLERANCE,
    IfInStructure, ImplyStructure, InequalityKind, InequalityStructure, MaxStructure, MinStructure,
    OrStructure, SigmoidStructure, SinStructure, binaryzation_core_relations,
    if_in_value_core_relations, imply_coupling_indicators, indicator_core_relations,
};
use crate::variable::VariableId;
use grb::constr::IneqExpr;
use grb::expr::LinExpr;
use grb::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;

/// ABS 原生写入的 schema 版本 / Schema version of the ABS native write.
///
/// schema 变化表示原生结构含义变化，恢复阶段必须拒绝旧指纹。
/// A schema change means the native structure's meaning changed and recovery must reject older
/// fingerprints.
pub const GUROBI_ABS_SCHEMA: &str = "functions-abs-1";

/// ABS 原生写入计划 / Plan for one native ABS write.
///
/// 只包含求解器无关的数据：函数名、结果列 ID 与参数列下标。
/// Contains solver-neutral data only: the function name, the result column ID and the argument column
/// index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列 `y` / Result column `y`
    pub result: VariableId,
    /// 参数列 `x` 在该模型列顺序中的下标，满足 `y = |x|`
    /// Index of the argument column `x` in this model's column order, with `y = |x|`
    ///
    /// 结构的输入多项式用**列下标**表示单项式（与中间模型的行同口径），因此计划只能给出下标；
    /// 由容器按列顺序解析成 SDK 变量。
    /// The structure's input polynomial expresses its monomial with a **column index** (the same
    /// convention as the intermediate model's rows), so the plan can only carry the index; the
    /// container resolves it to an SDK variable by column order.
    pub argument_index: usize,
}

/// 判断一个 ABS 结构是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 首批范围刻意收窄到**输入就是单个变量**（系数 1、无其它单项式、常数项 0）的情形：
/// Gurobi 的 `add_genconstr_abs` 只接受"结果变量 = |参数变量|"，而本项目的输入是任意线性多项式。
/// 通用仿射输入需要额外的桥接列与 `p = a·x + c` 约束，那会改变公开列并使原生路径不再更紧，
/// 因此在首批实现中明确拒绝并回退，而不是把不完整的关系写进模型。
///
/// 其它拒绝原因：结果列与参数列相同（自引用）、输入系数非 1（缩放会改变结果含义）。
///
/// Decide whether an ABS structure may be written natively and produce its write plan (SDK-free).
///
/// The first batch is deliberately narrow: the input must be **a single variable** (unit
/// coefficient, no other monomials, zero constant). Gurobi's `add_genconstr_abs` only accepts
/// "result variable = |argument variable|" while this project's input is an arbitrary linear
/// polynomial; a general affine input needs an extra bridge column and a `p = a·x + c` constraint,
/// which changes public columns and makes the native path no longer tighter. The first batch
/// therefore rejects such structures explicitly instead of writing an incomplete relation.
///
/// Other rejections: result and argument coincide (self-reference) and a non-unit input
/// coefficient, which would change the meaning of the result.
pub fn plan_abs_native(structure: &AbsStructure<f64>) -> std::result::Result<AbsNativePlan, FallbackReason> {
    let input = structure.input();
    let monomials = input.monomials();
    if monomials.len() != 1 {
        return Err(FallbackReason::Rejected(format!(
            "abs `{}` native lowering requires exactly one input monomial, got {}",
            structure.function_name(),
            monomials.len()
        )));
    }
    let monomial = &monomials[0];
    if *monomial.coefficient() != 1.0 {
        return Err(FallbackReason::Rejected(format!(
            "abs `{}` native lowering requires a unit input coefficient, got {}",
            structure.function_name(),
            monomial.coefficient()
        )));
    }
    if *input.constant_term() != 0.0 {
        return Err(FallbackReason::Rejected(format!(
            "abs `{}` native lowering requires a zero input constant, got {}",
            structure.function_name(),
            input.constant_term()
        )));
    }
    if structure.result() == structure.side() {
        return Err(FallbackReason::Rejected(format!(
            "abs `{}` native lowering requires distinct result and argument columns",
            structure.function_name()
        )));
    }
    if structure.result() == structure.side() {
        return Err(FallbackReason::Rejected(format!(
            "abs `{}` native lowering requires distinct result and argument columns",
            structure.function_name()
        )));
    }
    Ok(AbsNativePlan {
        name: structure.function_name().to_string(),
        result: structure.result().clone(),
        // 参数是输入单项式指向的**列**，不是 ABS 的分支指示列 `side`；后者只是回退公式里的
        // 选择器，把它当成参数会写出 `y = |side|` 这种语义完全错误的关系。
        // The argument is the **column** the input monomial points at, not the ABS branch selector
        // `side`: the latter is only the selector of the fallback rows, and using it as the argument
        // would write a semantically wrong relation such as `y = |side|`.
        argument_index: monomial.var_index(),
    })
}

/// 求解器容器：Gurobi 模型与"列 ID -> SDK 变量"的映射。
///
/// 变量在 native writer 运行之前已按中间模型的列顺序创建，因此容器的职责只是提供查询入口，
/// 而不是再次创建列——这样原生路径不会新增或删除任何公开列。
///
/// 容器**按值持有** Gurobi 模型：注册表要求容器类型满足 `'static`，而借用形式的容器带有生命周期
/// 参数，无法满足该约束。写入阶段结束后用 `into_parts` 取回模型继续装行求解。
///
/// Solver container: the Gurobi model plus the "column ID -> SDK variable" mapping.
///
/// Variables are created from the intermediate model's column order before native writers run, so
/// the container only offers lookups instead of creating columns again; the native path therefore
/// never adds or removes a public column.
///
/// The container **owns** the Gurobi model by value: the registry requires its container type to be
/// `'static`, which a borrowing container with a lifetime parameter cannot satisfy. `into_parts`
/// hands the model back afterwards so row loading and the solve continue on the same model.
pub struct GurobiNativeContainer {
    /// Gurobi 模型 / Gurobi model
    model: Model,
    /// 列 ID 到 SDK 变量的映射 / Mapping from column ID to SDK variable
    pub columns: HashMap<VariableId, Var>,
    /// 按列顺序排列的 SDK 变量 / SDK variables in column order
    columns_by_position: Vec<Var>,
}

impl Debug for GurobiNativeContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GurobiNativeContainer")
            .field("columns", &self.columns.len())
            .finish()
    }
}

impl GurobiNativeContainer {
    /// 创建容器 / Create a container.
    pub fn new(
        model: Model,
        columns: HashMap<VariableId, Var>,
        columns_by_position: Vec<Var>,
    ) -> Self {
        Self {
            model,
            columns,
            columns_by_position,
        }
    }

    /// 按列 ID 查找 SDK 变量 / Look up the SDK variable of a column ID.
    pub fn variable(&self, id: &VariableId) -> Option<Var> {
        self.columns.get(id).copied()
    }

    /// 按列下标查找 SDK 变量 / Look up the SDK variable of a column index.
    pub fn variable_at(&self, index: usize) -> Option<Var> {
        self.columns_by_position.get(index).copied()
    }

    /// 可变访问 Gurobi 模型 / Mutably access the Gurobi model.
    pub fn model_mut(&mut self) -> &mut Model {
        &mut self.model
    }

    /// 取回模型与列映射 / Take back the model and the column mapping.
    pub fn into_parts(self) -> (Model, HashMap<VariableId, Var>) {
        (self.model, self.columns)
    }
}

/// Gurobi 的 ABS 原生 writer / Gurobi's native ABS writer.
///
/// 接入方式：把本 writer 注册进 `NativeFunctionWriterRegistry<GurobiNativeContainer, f64>`，
/// 由 `MechanismModel::lower_deferred_functions` 统一调度。准入失败返回 `Fallback`，SDK 写入
/// 失败返回 `Err`，由调用方对整模型回退——与 Kotlin 的"写入失败丢弃整模型并重建 fallback"
/// 一致。
///
/// How to use it: register this writer in a
/// `NativeFunctionWriterRegistry<GurobiNativeContainer, f64>` and let
/// `MechanismModel::lower_deferred_functions` schedule it. A failed admission returns `Fallback`
/// while an SDK write failure returns `Err` so the caller can fall back for the whole model,
/// matching Kotlin's "drop the whole model and rebuild the fallback" behaviour.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiAbsWriter;

impl GurobiAbsWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiAbsWriter {
    fn name(&self) -> &str {
        "gurobi_abs"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure.as_any().downcast_ref::<AbsStructure<f64>>().is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let structure = request
                .structure
                .as_any()
                .downcast_ref::<AbsStructure<f64>>()
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "gurobi_abs writer received a non-abs structure".to_string(),
                    )
                })?;

            let plan = match plan_abs_native(structure) {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "abs `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 辅助列（分支指示列）必须仅被本函数自己的关系行引用：原生关系只约束结果列，不再约束
            // 分支列，因此一旦模型在别处引用该列，原生写入就会改变它的含义。
            // The helper column (the branch selector) must be referenced by this function's own rows
            // only: the native relation constrains the result column and no longer the branch column,
            // so a native write would change its meaning if the model references it elsewhere.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "abs `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let (Some(result_var), Some(argument_var)) = (
                container.variable(&plan.result),
                container.variable_at(plan.argument_index),
            ) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "abs `{}` native lowering could not find both columns in the Gurobi model",
                    plan.name
                ))));
                continue;
            };

            // 自引用检查只能在这里做：结果列是列 ID，参数列是列下标，只有解析成 SDK 变量后
            // 才能比较二者是否为同一列。
            // The self-reference check only belongs here: the result is a column ID while the argument
            // is a column index, and only after both are resolved to SDK variables can they be
            // compared.
            if result_var == argument_var {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "abs `{}` native lowering requires distinct result and argument columns",
                    plan.name
                ))));
                continue;
            }

            // SDK 写入失败直接返回错误：调用方必须对整模型回退，而不是保留半写入状态。
            // An SDK write failure returns an error: the caller must fall back for the whole model
            // instead of keeping a half-written state.
            container
                .model_mut()
                .add_genconstr_abs(&format!("{}_abs_native", plan.name), result_var, argument_var)
                .map_err(|error| {
                    ModelError::InvalidConstraint(format!(
                        "gurobi_abs writer failed to write abs `{}`: {error}",
                        plan.name
                    ))
                })?;

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_ABS_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// MAX 原生写入的 schema 版本 / Schema version of the native MAX write.
pub const GUROBI_MAX_SCHEMA: &str = "functions-max-1";
/// MIN 原生写入的 schema 版本 / Schema version of the native MIN write.
pub const GUROBI_MIN_SCHEMA: &str = "functions-min-1";

/// 极值（MAX/MIN）原生写入计划 / Plan for one native extremum write.
///
/// Gurobi 的 `add_genconstr_max/min` 形如 `result = max/min(operands) + constant`：候选只能整体
/// 提供一个共享常数，且每个操作数必须是**列**而不是一般仿射表达式。因此准入条件为：候选集非空、
/// 每个候选都是「系数为 1 的单个单项式」、所有候选共享同一个常数项（可为 0）。
///
/// Gurobi's `add_genconstr_max/min` reads `result = max/min(operands) + constant`: the candidates can
/// only contribute one shared constant and every operand must be a **column** rather than a general
/// affine expression. Admission therefore requires a non-empty candidate set where every candidate is
/// a single unit-coefficient monomial and all candidates share the same constant (possibly zero).
#[derive(Debug, Clone, PartialEq)]
pub struct ExtremumNativePlan {
    /// 函数名称 / Function name
    pub name: String,
    /// 结果列 / Result column
    pub result: VariableId,
    /// 操作数列在列视图中的位置 / Positions of the operand columns in the column view
    pub operand_indices: Vec<usize>,
    /// 共享常数项（0 或不存在时为 `None`）/ Shared constant (`None` when absent or zero)
    pub constant: Option<f64>,
    /// 是否为 MIN / Whether this is a MIN write
    pub minimum: bool,
}

/// 规划一次极值原生写入 / Plan one native extremum write.
fn plan_extremum(
    name: &str,
    result: &VariableId,
    polynomials: &[Linear<f64>],
    minimum: bool,
) -> std::result::Result<ExtremumNativePlan, FallbackReason> {
    let kind = if minimum { "min" } else { "max" };
    if polynomials.is_empty() {
        return Err(FallbackReason::Rejected(format!(
            "extremum `{name}` native lowering requires at least one candidate ({kind})"
        )));
    }

    let mut operand_indices = Vec::with_capacity(polynomials.len());
    let mut shared_constant: Option<f64> = None;
    for polynomial in polynomials {
        let monomials = polynomial.monomials();
        if monomials.len() != 1 || *monomials[0].coefficient() != 1.0 {
            return Err(FallbackReason::Rejected(format!(
                "extremum `{name}` native lowering requires every candidate to be a single unit-coefficient monomial ({kind})"
            )));
        }
        let value = *polynomial.constant_term();
        match shared_constant {
            None => shared_constant = Some(value),
            Some(existing) if existing != value => {
                return Err(FallbackReason::Rejected(format!(
                    "extremum `{name}` native lowering requires a shared candidate constant, got {existing} and {value} ({kind})"
                )));
            }
            Some(_) => {}
        }
        operand_indices.push(monomials[0].var_index());
    }

    let constant = match shared_constant {
        Some(value) if value != 0.0 => Some(value),
        _ => None,
    };

    Ok(ExtremumNativePlan {
        name: name.to_string(),
        result: result.clone(),
        operand_indices,
        constant,
        minimum,
    })
}

/// 规划 MAX 的原生写入 / Plan the native write of a MAX structure.
pub fn plan_max_native(
    structure: &MaxStructure<f64>,
) -> std::result::Result<ExtremumNativePlan, FallbackReason> {
    plan_extremum(
        structure.function_name(),
        structure.result(),
        structure.candidate_polynomials(),
        false,
    )
}

/// 规划 MIN 的原生写入 / Plan the native write of a MIN structure.
pub fn plan_min_native(
    structure: &MinStructure<f64>,
) -> std::result::Result<ExtremumNativePlan, FallbackReason> {
    plan_extremum(
        structure.function_name(),
        structure.result(),
        structure.candidate_polynomials(),
        true,
    )
}

/// Gurobi 的 MAX/MIN 原生 writer / Gurobi's native extremum writer.
///
/// 与 ABS writer 遵循同一套门控：结果列被固定、或辅助列（候选选择器）被外部引用时一律回退，
/// 因为原生关系只约束结果列，不会继续约束这些辅助列。
///
/// Follows the same gates as the ABS writer: a fixed result column or externally referenced helper
/// columns (the candidate selectors) both force a fallback, because the native relation only
/// constrains the result column and would leave those helpers meaningless.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiExtremumWriter {
    minimum: bool,
}

impl GurobiExtremumWriter {
    /// 创建 MAX writer / Create the MAX writer.
    pub fn maximum() -> Self {
        Self { minimum: false }
    }

    /// 创建 MIN writer / Create the MIN writer.
    pub fn minimum() -> Self {
        Self { minimum: true }
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiExtremumWriter {
    fn name(&self) -> &str {
        if self.minimum {
            "gurobi_min"
        } else {
            "gurobi_max"
        }
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        if self.minimum {
            structure
                .as_any()
                .downcast_ref::<MinStructure<f64>>()
                .is_some()
        } else {
            structure
                .as_any()
                .downcast_ref::<MaxStructure<f64>>()
                .is_some()
        }
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        let kind = if self.minimum { "min" } else { "max" };
        let schema = if self.minimum {
            GUROBI_MIN_SCHEMA
        } else {
            GUROBI_MAX_SCHEMA
        };
        let mut outcomes = Vec::with_capacity(requests.len());

        for request in requests {
            let plan = if self.minimum {
                let structure = request
                    .structure
                    .as_any()
                    .downcast_ref::<MinStructure<f64>>()
                    .ok_or_else(|| {
                        ModelError::InvalidConstraint(
                            "gurobi_min writer received a structure that is not a MIN structure"
                                .to_string(),
                        )
                    })?;
                plan_min_native(structure)
            } else {
                let structure = request
                    .structure
                    .as_any()
                    .downcast_ref::<MaxStructure<f64>>()
                    .ok_or_else(|| {
                        ModelError::InvalidConstraint(
                            "gurobi_max writer received a structure that is not a MAX structure"
                                .to_string(),
                        )
                    })?;
                plan_max_native(structure)
            };

            let plan = match plan {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                    format!("{kind} `{}` native lowering rejected a fixed result column", plan.name),
                )));
                continue;
            }

            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                    format!(
                        "{kind} `{}` native lowering rejected externally referenced helper columns",
                        plan.name
                    ),
                )));
                continue;
            }

            let Some(result_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                    format!("{kind} `{}` result column is missing from the solve model", plan.name),
                )));
                continue;
            };

            let mut operand_vars = Vec::with_capacity(plan.operand_indices.len());
            let mut missing_operand = false;
            for index in &plan.operand_indices {
                match container.variable_at(*index) {
                    Some(var) => operand_vars.push(var),
                    None => {
                        missing_operand = true;
                        break;
                    }
                }
            }
            if missing_operand {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                    format!("{kind} `{}` operand column is missing from the solve model", plan.name),
                )));
                continue;
            }

            // 结果列不能同时作为操作数：原生关系要求比较对象与结果不同。
            // The result column must not be one of the operands: the native relation compares distinct
            // objects and a self-comparison would silently encode a different relation.
            if operand_vars.contains(&result_var) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(
                    format!(
                        "{kind} `{}` native lowering requires operand columns distinct from the result",
                        plan.name
                    ),
                )));
                continue;
            }

            let write = if self.minimum {
                container.model_mut().add_genconstr_min(
                    &format!("{}_min_native", plan.name),
                    result_var,
                    operand_vars,
                    plan.constant,
                )
            } else {
                container.model_mut().add_genconstr_max(
                    &format!("{}_max_native", plan.name),
                    result_var,
                    operand_vars,
                    plan.constant,
                )
            };
            write.map_err(|error| {
                ModelError::InvalidConstraint(format!(
                    "gurobi_{kind} writer failed to write {kind} `{}`: {error}",
                    plan.name
                ))
            })?;

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                schema,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// PWL（分段线性形状）原生写入的 schema 版本 / Schema version of the native PWL write.
///
/// Sin/Cos/Sigmoid 的点表语义一致（都是 `y = f(x)` 的分段线性插值），因此三者共用一个 writer 与
/// 一个 schema；schema 变化表示原生结构的含义变化，恢复阶段必须拒绝旧指纹。
///
/// Sin/Cos/Sigmoid share one writer and one schema because their point tables have the same
/// semantics (a piecewise-linear interpolation of `y = f(x)`); a schema change means the native
/// structure's meaning changed and recovery must reject older fingerprints.
pub const GUROBI_PWL_SCHEMA: &str = "functions-pwl-1";

/// 范围证明允许的边界容差 / Tolerance allowed by the range proof.
///
/// 输入列的实际界只要不超出 `[x0, xn]` 超过该容差即可原生写入；容差吸收声明边界的十进制舍入。
/// The input column's actual bounds may exceed `[x0, xn]` by at most this tolerance for a native
/// write; the tolerance absorbs decimal rounding of declared bounds.
pub const GUROBI_PWL_RANGE_TOLERANCE: f64 = 1e-9;

/// PWL 原生写入计划 / Plan for one native PWL write.
///
/// 只包含求解器无关的数据：形状名、函数名、结果列 ID、输入列下标与点表 `(x_i, f(x_i))`。
/// Contains solver-neutral data only: the shape name, the function name, the result column ID, the
/// input column index and the point table `(x_i, f(x_i))`.
#[derive(Debug, Clone, PartialEq)]
pub struct PwlNativePlan {
    /// 形状名称（`sin` / `cos` / `sigmoid`），仅用于诊断信息 / Shape name, diagnostics only
    pub shape: &'static str,
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列 `y` / Result column `y`
    pub result: VariableId,
    /// 输入列 `x` 在该模型列顺序中的下标，满足 `y = f(x)`
    /// Index of the input column `x` in this model's column order, with `y = f(x)`
    ///
    /// 与 ABS 计划同理：结构的输入多项式用**列下标**表示单项式（与中间模型的行同口径），因此计划
    /// 只能给出下标；由容器按列顺序解析成 SDK 变量。
    /// As with the ABS plan: the structure's input polynomial expresses its monomial with a
    /// **column index** (the same convention as the intermediate model's rows), so the plan can only
    /// carry the index; the container resolves it to an SDK variable by column order.
    pub argument_index: usize,
    /// 分段线性点表 `(x_i, f(x_i))`，x 严格递增且至少两点
    /// Piecewise-linear point table `(x_i, f(x_i))` with strictly increasing x and at least two points
    pub points: Vec<(f64, f64)>,
}

/// 判断一个分段线性形状是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 首批范围与 ABS/MAX/MIN 一致地收窄到**输入就是单个变量**（系数 1、无其它单项式、常数项 0）的
/// 情形：Gurobi 的 `add_genconstr_pwl` 只接受"结果变量 = f(参数变量)"，而本项目的输入是任意线性
/// 多项式。一般仿射输入 `a·x + c` 需要额外的桥接列与 `p = a·x + c` 约束，那会改变公开列并使原生
/// 路径不再更紧，因此在首批实现中明确拒绝并回退，而不是把不完整的关系写进模型。
///
/// 点表本身也必须可写：至少两点、x 严格递增且所有坐标有限；否则 Gurobi 无法定义分段线性函数，
/// 同样回退而不是写出降级的近似。
///
/// 注意：**本函数不做范围证明**。原生 PWL 在输入超出 `[x0, xn]` 时会外推，而即时展开会把输入夹在
/// 断点区间内（`{}_x_lb` / `{}_x_ub` 两行），二者语义不同；证明需要读 SDK 模型里的实际界，因此由
/// writer 在解析出列之后完成（见 [`GurobiPwlWriter::write_batch`]）。
///
/// Decide whether a piecewise-linear shape may be written natively and produce its write plan
/// (SDK-free).
///
/// As with ABS/MAX/MIN, the first batch is deliberately narrow: the input must be **a single
/// variable** (unit coefficient, no other monomials, zero constant). Gurobi's `add_genconstr_pwl`
/// only accepts "result variable = f(argument variable)" while this project's input is an arbitrary
/// linear polynomial; a general affine input `a·x + c` needs an extra bridge column and a
/// `p = a·x + c` constraint, which changes public columns and makes the native path no longer
/// tighter. The first batch therefore rejects such structures explicitly instead of writing an
/// incomplete relation.
///
/// The point table must be writable as well: at least two points, strictly increasing x and finite
/// coordinates; otherwise Gurobi cannot define the piecewise-linear function and the structure falls
/// back instead of being written as a degraded approximation.
///
/// Note that this function performs **no range proof**. A native PWL extrapolates when the input
/// leaves `[x0, xn]` while eager expansion clamps the input into the breakpoint interval (the
/// `{}_x_lb` / `{}_x_ub` rows), so the two have different semantics; the proof needs the actual
/// bounds from the SDK model and therefore happens in the writer once the columns are resolved (see
/// [`GurobiPwlWriter::write_batch`]).
pub fn plan_pwl_native(
    shape: &'static str,
    name: &str,
    result: &VariableId,
    input: &Linear<f64>,
    points: Vec<(f64, f64)>,
) -> std::result::Result<PwlNativePlan, FallbackReason> {
    let monomials = input.monomials();
    if monomials.len() != 1 {
        return Err(FallbackReason::Rejected(format!(
            "{shape} `{name}` native PWL lowering requires exactly one input monomial, got {}",
            monomials.len()
        )));
    }
    let monomial = &monomials[0];
    if *monomial.coefficient() != 1.0 {
        return Err(FallbackReason::Rejected(format!(
            "{shape} `{name}` native PWL lowering requires a unit input coefficient, got {}",
            monomial.coefficient()
        )));
    }
    if *input.constant_term() != 0.0 {
        return Err(FallbackReason::Rejected(format!(
            "{shape} `{name}` native PWL lowering requires a zero input constant, got {}",
            input.constant_term()
        )));
    }
    if points.len() < 2 {
        return Err(FallbackReason::Rejected(format!(
            "{shape} `{name}` native PWL lowering requires at least two points, got {}",
            points.len()
        )));
    }
    for (index, (x, y)) in points.iter().enumerate() {
        if !x.is_finite() || !y.is_finite() {
            return Err(FallbackReason::Rejected(format!(
                "{shape} `{name}` native PWL lowering requires finite point coordinates, point {index} is ({x}, {y})"
            )));
        }
    }
    for index in 1..points.len() {
        let previous = points[index - 1].0;
        let current = points[index].0;
        if current <= previous {
            return Err(FallbackReason::Rejected(format!(
                "{shape} `{name}` native PWL lowering requires strictly increasing point x values, got {previous} then {current} at index {index}"
            )));
        }
    }

    Ok(PwlNativePlan {
        shape,
        name: name.to_string(),
        result: result.clone(),
        // 输入是 `1 · x_k + 0`，所以输入列就是该单项式指向的列。
        // The input is `1 · x_k + 0`, so the input column is exactly the column the monomial points at.
        argument_index: monomial.var_index(),
        points,
    })
}

/// 规划 SIN 的原生写入 / Plan the native write of a SIN structure.
pub fn plan_sin_native(
    structure: &SinStructure<f64>,
) -> std::result::Result<PwlNativePlan, FallbackReason> {
    plan_pwl_native(
        "sin",
        structure.name(),
        structure.result(),
        structure.input_polynomial(),
        structure.points(),
    )
}

/// 规划 COS 的原生写入 / Plan the native write of a COS structure.
pub fn plan_cos_native(
    structure: &CosStructure<f64>,
) -> std::result::Result<PwlNativePlan, FallbackReason> {
    plan_pwl_native(
        "cos",
        structure.name(),
        structure.result(),
        structure.input_polynomial(),
        structure.points(),
    )
}

/// 规划 SIGMOID 的原生写入 / Plan the native write of a SIGMOID structure.
pub fn plan_sigmoid_native(
    structure: &SigmoidStructure<f64>,
) -> std::result::Result<PwlNativePlan, FallbackReason> {
    plan_pwl_native(
        "sigmoid",
        structure.name(),
        structure.result(),
        structure.input_polynomial(),
        structure.points(),
    )
}

/// Gurobi 的 PWL 原生 writer / Gurobi's native piecewise-linear writer.
///
/// 服务 Sin/Cos/Sigmoid 三种分段线性形状：它们的延迟结构都是"λ 凸组合 + 分段选择"，即时展开用
/// `x = Σ λ_i x_i` 把输入夹在断点区间内，而 Gurobi 的 `add_genconstr_pwl` 直接表达
/// `y = f(x)`。两者只有在输入列的实际界落在 `[x0, xn]` 内时才等价，因此本 writer 的独有门控是
/// **范围证明**：从 SDK 模型读输入列的 `LB` / `UB`，越界即回退。
///
/// 与 ABS/MAX/MIN 共用其余门控：结果列被固定、辅助列（分段偏移列 / 选择器列 / λ 权重列）被外部
/// 引用、结果列或输入列在求解模型中缺失、结果列与输入列相同，都回退。绝不写语义不完整的关系；
/// 真正的 SDK 写入失败返回 `Err`，由调用方对整模型回退。
///
/// Serves the three piecewise-linear shapes Sin/Cos/Sigmoid: their deferred structures are all
/// "λ convex combination plus segment selection", and eager expansion uses `x = Σ λ_i x_i` to clamp
/// the input into the breakpoint interval, while Gurobi's `add_genconstr_pwl` expresses `y = f(x)`
/// directly. The two are equivalent only when the input column's actual bounds lie inside
/// `[x0, xn]`, so this writer's distinctive gate is a **range proof**: it reads the input column's
/// `LB` / `UB` from the SDK model and falls back when the bounds leave the interval.
///
/// The remaining gates are shared with ABS/MAX/MIN: a fixed result column, externally referenced
/// helper columns (piecewise offsets, selectors and λ weights), a result or input column missing
/// from the solve model, and coinciding result and input columns all force a fallback. An incomplete
/// relation is never written, and a genuine SDK write failure returns `Err` so the caller can fall
/// back for the whole model.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiPwlWriter;

impl GurobiPwlWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

/// 该结构是否是本 writer 服务的分段线性形状 / Whether the structure is a piecewise-linear shape this writer serves.
fn is_pwl_structure(structure: &dyn DeferredFunctionStructure<f64>) -> bool {
    let any = structure.as_any();
    any.downcast_ref::<SinStructure<f64>>().is_some()
        || any.downcast_ref::<CosStructure<f64>>().is_some()
        || any.downcast_ref::<SigmoidStructure<f64>>().is_some()
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiPwlWriter {
    fn name(&self) -> &str {
        "gurobi_pwl"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        is_pwl_structure(structure)
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        // 范围证明要读 SDK 里的列属性，而刚建好的列在 Gurobi 的 lazy update 模式下仍是 pending
        // 对象（属性读取会因为对象尚未落地而失败），因此先把待定变更落地一次。`update()` 只推送尚未
        // 生效的变更，对之后的装行与求解没有语义影响。
        //
        // The range proof reads column attributes from the SDK, but freshly created columns are still
        // pending objects under Gurobi's lazy update mode (an attribute read fails while the object is
        // not materialized), so the pending changes are flushed once first. `update()` only pushes
        // not-yet-applied changes and has no semantic effect on the later row loading and solve.
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_pwl writer failed to flush pending model changes before the range proof: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let any = request.structure.as_any();
            let plan = if let Some(structure) = any.downcast_ref::<SinStructure<f64>>() {
                plan_sin_native(structure)
            } else if let Some(structure) = any.downcast_ref::<CosStructure<f64>>() {
                plan_cos_native(structure)
            } else if let Some(structure) = any.downcast_ref::<SigmoidStructure<f64>>() {
                plan_sigmoid_native(structure)
            } else {
                return Err(ModelError::InvalidConstraint(
                    "gurobi_pwl writer received a structure that is not a piecewise-linear shape"
                        .to_string(),
                )
                .into());
            };

            let plan = match plan {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };
            let shape = plan.shape;

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{shape} `{}` native PWL lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 辅助列（分段偏移列 / 选择器列 / λ 权重列）必须仅被本函数自己的关系行引用：原生关系
            // 只约束 `y = f(x)`，不再约束这些辅助列，一旦模型在别处引用它们，原生写入就会改变它们
            // 的含义。
            // The helper columns (piecewise offsets, selectors and λ weights) must be referenced by
            // this function's own rows only: the native relation constrains `y = f(x)` and no longer
            // those helpers, so a native write would change their meaning if the model references
            // them elsewhere.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{shape} `{}` native PWL lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let (Some(result_var), Some(argument_var)) = (
                container.variable(&plan.result),
                container.variable_at(plan.argument_index),
            ) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{shape} `{}` native PWL lowering could not find both columns in the Gurobi model",
                    plan.name
                ))));
                continue;
            };

            // 自引用检查只能在解析出 SDK 变量之后做：结果列是列 ID，输入列是列下标。
            // The self-reference check only belongs after both columns are resolved to SDK variables:
            // the result is a column ID while the input is a column index.
            if result_var == argument_var {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{shape} `{}` native PWL lowering requires distinct result and input columns",
                    plan.name
                ))));
                continue;
            }

            // 范围证明：读输入列在求解模型里的**实际**上下界。读不到界意味着无法证明等价性，
            // 保守回退而不是冒险写入。
            // Range proof: read the input column's **actual** bounds from the solve model. Failing to
            // read them means equivalence cannot be proven, so the writer conservatively falls back
            // instead of writing on faith.
            let bounds = {
                let model = container.model_mut();
                match model.get_obj_attr(attr::LB, &argument_var) {
                    Ok(lb) => model
                        .get_obj_attr(attr::UB, &argument_var)
                        .map(|ub| (lb, ub))
                        .map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                }
            };
            let (lower_bound, upper_bound) = match bounds {
                Ok(bounds) => bounds,
                Err(error) => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "{shape} `{}` native PWL lowering could not read the input column bounds from the Gurobi model: {error}",
                        plan.name
                    ))));
                    continue;
                }
            };

            // 点表的 x 范围就是即时展开把输入夹住的那个区间。
            // The point table's x range is exactly the interval eager expansion clamps the input into.
            let (domain_min, _) = plan.points[0];
            let (domain_max, _) = plan.points[plan.points.len() - 1];
            if lower_bound < domain_min - GUROBI_PWL_RANGE_TOLERANCE
                || upper_bound > domain_max + GUROBI_PWL_RANGE_TOLERANCE
            {
                // 否则原生 PWL 会**外推**，而即时路径会把输入夹在断点区间 `[x0, xn]` 内（`x_lb` /
                // `x_ub` 两行），二者语义不同，因此必须回退而不能写。
                // Otherwise the native PWL would **extrapolate** while the eager path clamps the
                // input into the breakpoint interval `[x0, xn]` (the `x_lb` / `x_ub` rows); the two
                // have different semantics, so the writer must fall back rather than write.
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{shape} `{}` native PWL lowering requires the input column bounds inside [{domain_min}, {domain_max}], got [{lower_bound}, {upper_bound}]",
                    plan.name
                ))));
                continue;
            }

            // SDK 写入失败直接返回错误：调用方必须对整模型回退，而不是保留半写入状态。
            // An SDK write failure returns an error: the caller must fall back for the whole model
            // instead of keeping a half-written state.
            container
                .model_mut()
                .add_genconstr_pwl(
                    &format!("{}_pwl_native", plan.name),
                    argument_var,
                    result_var,
                    plan.points.iter().copied(),
                )
                .map_err(|error| {
                    ModelError::InvalidConstraint(format!(
                        "gurobi_pwl writer failed to write {shape} `{}`: {error}",
                        plan.name
                    ))
                })?;

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_PWL_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// 关系指示（条件形状）原生写入的 schema 版本 / Schema version of the native relation-indicator write.
///
/// schema 变化表示原生结构的含义变化，恢复阶段必须拒绝旧指纹。
/// A schema change means the native structure's meaning changed and recovery must reject older
/// fingerprints.
pub const GUROBI_INDICATOR_SCHEMA: &str = "functions-indicator-1";

/// Big-M 冗余证明允许的容差 / Tolerance allowed by the Big-M redundancy proof.
///
/// 关系指示的即时展开是「核心行 + Big-M 松弛行」，而原生接口只能写核心行（指示列取真/取假各一条
/// `add_genconstr_indicator`）。因此必须证明那两条松弛行在条件变量的**实际盒**上恒成立，否则原生
/// 路径会比即时展开更松（可行解集合变大）。
///
/// 容差吸收两件事：Big-M 由 `infer_linear_shifted_abs_bound_from_tokens` 取到盒的绝对界时，松弛行
/// 在盒角上恰好差一个 ε（1e-10）；以及十进制界的舍入。取值与 PWL 范围证明的容差一致。
///
/// The relation indicator's eager expansion is "core rows plus Big-M relaxed rows" while the native
/// interface can only write the core rows (one `add_genconstr_indicator` per indicator value). The two
/// relaxed rows must therefore be proven to hold everywhere on the condition variables' **actual
/// box**, otherwise the native path would be looser than eager expansion (a larger feasible set).
///
/// The tolerance covers two effects: when the Big-M comes from
/// `infer_linear_shifted_abs_bound_from_tokens` picking the box's absolute bound, a relaxed row misses
/// by exactly one ε (1e-10) at a box corner; and decimal rounding of the declared bounds. The value
/// matches the PWL range proof's tolerance.
pub const GUROBI_INDICATOR_BIG_M_TOLERANCE: f64 = 1e-9;

/// 关系指示（条件形状）原生写入计划 / Plan for one native relation-indicator write.
///
/// 原生接口形如「指示列 `ind == ind_val` 时线性约束 `con` 成立」，因此每条即时核心行对应一条
/// `add_genconstr_indicator`：
///
/// - `y = 1` ⇒ `when_true`（即时展开里**不含 Big-M** 的那条行）；
/// - `y = 0` ⇒ `when_false`（另一条即时行去掉 Big-M 后的同一形式）。
///
/// 条件写在平移后的线性式 `s = Σ c_k x_k + constant` 上，`constant = left_constant - right`，
/// 与即时展开 `build_constraint` 用的平移量完全相同；严格性 ε 与即时路径共用
/// [`INDICATOR_TOLERANCE`]，本计划不引入任何自造容差。行里的 Big-M 松弛由
/// [`Self::prove_big_m_relaxation`] 单独证明。
///
/// The native interface reads "the linear constraint `con` holds when the indicator column
/// `ind == ind_val`", so every eager core row maps to one `add_genconstr_indicator`:
///
/// - `y = 1` ⇒ `when_true` (the eager row that carries **no Big-M**);
/// - `y = 0` ⇒ `when_false` (the other eager row with the Big-M removed).
///
/// Conditions are written on the shifted linear form `s = Σ c_k x_k + constant` with
/// `constant = left_constant - right`, exactly the shift the eager `build_constraint` uses, and the
/// strictness ε comes from the eager path's [`INDICATOR_TOLERANCE`] instead of any tolerance invented
/// here. The rows' Big-M relaxations are proven separately by [`Self::prove_big_m_relaxation`].
#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 指示列 `y`，同时是本函数的**结果列** / Indicator column `y`, which is also the result column
    pub result: VariableId,
    /// 平移后线性式的单项式：条件列下标 + 系数
    /// Monomials of the shifted linear form: condition column index + coefficient
    pub coefficients: Vec<(usize, f64)>,
    /// 平移后线性式的常数项 `left_constant - right` / Constant of the shifted form
    pub constant: f64,
    /// 结构创建时固定的 Big-M / Big-M fixed when the structure was created
    pub big_m: f64,
    /// 即时展开的严格性容差 ε（与即时路径共用同一常量）
    /// Strictness tolerance ε of the eager expansion, shared with the eager path
    pub tolerance: f64,
    /// 指示列取真时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 1`
    pub when_true: (ConstraintRelation, f64),
    /// 指示列取假时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 0`
    pub when_false: (ConstraintRelation, f64),
    /// 需要证明的下侧松弛 `s >= relaxed_lower_rhs - M`
    /// Lower relaxation to prove: `s >= relaxed_lower_rhs - M`
    pub relaxed_lower_rhs: f64,
    /// 需要证明的上侧松弛 `s <= M - relaxed_upper_rhs`
    /// Upper relaxation to prove: `s <= M - relaxed_upper_rhs`
    pub relaxed_upper_rhs: f64,
}

impl IndicatorNativePlan {
    /// 证明即时展开里被 Big-M 松弛掉的两条行在条件变量的盒 `[s_min, s_max]` 上恒成立。
    ///
    /// `s = Σ c_k x_k + constant` 的实际取值范围由 writer 从求解模型的列界算出；只要两条松弛行在
    /// 整个盒上都成立，即时展开相对原生两条 indicator 就没有额外约束，两者的可行解集合一致（差异
    /// 仅在容差量级）。
    ///
    /// Prove that both rows eager expansion relaxes through the Big-M hold everywhere on the
    /// condition variables' box `[s_min, s_max]`.
    ///
    /// The writer derives the actual range of `s = Σ c_k x_k + constant` from the solve model's column
    /// bounds; as long as both relaxed rows hold on the whole box, eager expansion adds no constraint
    /// beyond the native pair of indicator constraints and the two feasible sets coincide (up to the
    /// proof's tolerance).
    pub fn prove_big_m_relaxation(
        &self,
        s_min: f64,
        s_max: f64,
    ) -> std::result::Result<(), String> {
        if !s_min.is_finite() || !s_max.is_finite() || s_min > s_max {
            return Err(format!(
                "relation indicator `{}` has no finite condition domain, got [{s_min}, {s_max}]",
                self.name
            ));
        }
        if self.big_m + s_min < self.relaxed_lower_rhs - GUROBI_INDICATOR_BIG_M_TOLERANCE {
            return Err(format!(
                "relation indicator `{}` big-M {} does not imply the eager lower relaxation `s >= {} - M` on [{s_min}, {s_max}]",
                self.name, self.big_m, self.relaxed_lower_rhs
            ));
        }
        if self.big_m - s_max < self.relaxed_upper_rhs - GUROBI_INDICATOR_BIG_M_TOLERANCE {
            return Err(format!(
                "relation indicator `{}` big-M {} does not imply the eager upper relaxation `s <= M - {}` on [{s_min}, {s_max}]",
                self.name, self.big_m, self.relaxed_upper_rhs
            ));
        }
        Ok(())
    }
}

/// 判断一个关系指示结构是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 允许的 kinds 是 `<=`、`>=`、`<`、`>`：它们的即时展开在指示列取每个值时恰好有一条**不含
/// Big-M** 的行，原生 writer 把这两条行分别写成 `add_genconstr_indicator`。返回的核心关系与松弛量
/// 直接来自符号文件里的 [`indicator_core_relations`]，因此严格性 ε 与即时路径逐位相同。
///
/// `=` / `!=` **明确拒绝**：它们的取假侧是析取（`s <= -boundary` 或 `s >= boundary`），即时展开靠
/// 辅助 `side` 二元列做情形分裂，两条 indicator 无法表达；即使额外补上指示列与 side 列的耦合行，
/// 那也不再是「原生写入」，而且会改变列的使用语境。拒绝即保持 EAGER 展开。
///
/// 其它拒绝原因：Big-M 非正或非有限、左右常数或条件系数非有限、条件里没有任何变量项（在 SDK 里
/// 会退化成一条没有变量的空表达式一般约束）。**本函数不做盒证明**：松弛行的冗余需要读 SDK 列界，
/// 由 writer 在解析出列之后完成（见 [`GurobiIndicatorWriter::write_batch`]）。
///
/// Decide whether a relation-indicator structure may be written natively and produce its write plan
/// (SDK-free).
///
/// The admitted kinds are `<=`, `>=`, `<` and `>`: for each value of the indicator column their eager
/// expansion has exactly one row **without** a Big-M term, and the native writer turns those two rows
/// into `add_genconstr_indicator` calls. The returned core relations and relaxations come straight from
/// the symbol file's [`indicator_core_relations`], so the strictness ε is bit-identical to the eager
/// path's.
///
/// `=` / `!=` are **rejected explicitly**: their false side is a disjunction
/// (`s <= -boundary` or `s >= boundary`) that eager expansion splits with the auxiliary `side` binary
/// column, and two indicator constraints cannot express it; adding a coupling row between the
/// indicator and the side column would no longer be a native write and would change that column's usage
/// context. Rejecting keeps the structure on the EAGER expansion.
///
/// Other rejections: a non-positive or non-finite Big-M, non-finite constants or condition
/// coefficients, and a condition without any variable term (which would degrade into an empty general
/// constraint in the SDK). This function performs **no box proof**: proving the relaxed rows redundant
/// needs the SDK column bounds and therefore happens in the writer once the columns are resolved (see
/// [`GurobiIndicatorWriter::write_batch`]).
pub fn plan_indicator_native(
    structure: &InequalityStructure<f64>,
) -> std::result::Result<IndicatorNativePlan, FallbackReason> {
    plan_indicator_from_parts(
        structure.name(),
        structure.inequality_kind(),
        structure.big_m(),
        structure.left_polynomial(),
        *structure.right_value(),
        structure.result().clone(),
    )
}

/// 从关系指示的原始数据规划一次原生写入 / Plan one native write from a relation indicator's raw data
///
/// 与 [`plan_indicator_native`] 共用同一条准入与映射逻辑，因此**嵌套在其它结构里的**关系指示器（例如
/// 蕴含结构自带的前提/结论子指示器）也能拿到同一份计划：严格性 ε 依旧取自 [`indicator_core_relations`]，
/// 不会出现第二套映射。`name` 同时用作原生约束名前缀，调用方需保证不同子指示器的前缀不同。
///
/// Shares the very same admission and mapping logic as [`plan_indicator_native`], so a relation indicator
/// **nested inside another structure** (for example the premise/consequence sub-indicators an implication
/// carries) gets the same plan: the strictness ε still comes from [`indicator_core_relations`] and there is
/// no second mapping. `name` doubles as the native constraint name prefix, so callers must keep the prefixes
/// of different sub-indicators distinct.
pub fn plan_indicator_from_parts(
    name: &str,
    kind: InequalityKind,
    big_m: f64,
    left: &Linear<f64>,
    right: f64,
    result: VariableId,
) -> std::result::Result<IndicatorNativePlan, FallbackReason> {
    let Some(core) = indicator_core_relations(kind) else {
        return Err(FallbackReason::Rejected(format!(
            "relation indicator `{name}` native lowering cannot express kind {kind:?}: `=`/`!=` need the auxiliary side column for a case split, which two indicator constraints cannot carry"
        )));
    };
    if !big_m.is_finite() || big_m <= 0.0 {
        return Err(FallbackReason::Rejected(format!(
            "relation indicator `{name}` native lowering requires a positive finite big-M, got {big_m}"
        )));
    }

    if !right.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "relation indicator `{name}` native lowering requires a finite right-hand side, got {right}"
        )));
    }

    let mut coefficients = Vec::with_capacity(left.monomials().len());
    for monomial in left.monomials() {
        let coefficient = *monomial.coefficient();
        if !coefficient.is_finite() {
            return Err(FallbackReason::Rejected(format!(
                "relation indicator `{name}` native lowering requires finite condition coefficients, got {coefficient}"
            )));
        }
        coefficients.push((monomial.var_index(), coefficient));
    }
    if !coefficients
        .iter()
        .any(|(_, coefficient)| *coefficient != 0.0)
    {
        return Err(FallbackReason::Rejected(format!(
            "relation indicator `{name}` native lowering requires at least one variable term in the condition, got {} monomials with no non-zero coefficient",
            coefficients.len()
        )));
    }

    // 平移量与即时展开的 `build_constraint` 完全一致：`shifted_constant = left_constant - right`。
    // The shift is identical to the eager `build_constraint`'s:
    // `shifted_constant = left_constant - right`.
    let constant = *left.constant_term() - right;
    if !constant.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "relation indicator `{name}` native lowering requires a finite shifted constant, got {constant}"
        )));
    }

    Ok(IndicatorNativePlan {
        name: name.to_string(),
        result,
        coefficients,
        constant,
        big_m,
        // ε 直接取即时路径的同一常量，绝不在本模块另取一个。
        // ε is the eager path's very constant; this module never picks another one.
        tolerance: INDICATOR_TOLERANCE,
        when_true: core.when_true,
        when_false: core.when_false,
        relaxed_lower_rhs: core.relaxed_lower_rhs,
        relaxed_upper_rhs: core.relaxed_upper_rhs,
    })
}

/// 把核心关系翻译成 Gurobi 的比较方向 / Translate a core relation into Gurobi's comparison sense.
fn indicator_sense(relation: ConstraintRelation) -> ConstrSense {
    match relation {
        ConstraintRelation::LessEqual => ConstrSense::Less,
        ConstraintRelation::GreaterEqual => ConstrSense::Greater,
        ConstraintRelation::Equal => ConstrSense::Equal,
    }
}

/// 构造条件行 `Σ c_k x_k + constant REL rhs` / Build the condition row `Σ c_k x_k + constant REL rhs`
fn indicator_condition(
    terms: &[(Var, f64)],
    constant: f64,
    relation: ConstraintRelation,
    rhs: f64,
) -> IneqExpr {
    let mut expression = LinExpr::new();
    for (var, coefficient) in terms {
        expression.add_term(*coefficient, *var);
    }
    expression.add_constant(constant);
    IneqExpr {
        lhs: Expr::from(expression),
        sense: indicator_sense(relation),
        rhs: Expr::Constant(rhs),
    }
}

/// Gurobi 的关系指示（条件形状）原生 writer / Gurobi's native relation-indicator writer.
///
/// 服务 [`InequalityStructure`]（`ospf-rust-core/src/symbol/functions/inequality.rs`）：它的即时展开
/// 是「指示列 = 1 ⇒ 核心关系；= 0 ⇒ 另一条核心关系」，外加两条 Big-M 松弛行；本 writer 用两条
/// `add_genconstr_indicator` 写核心关系，并在写入前证明松弛行冗余。
///
/// 与 ABS/MAX/MIN/PWL 共用其余门控：结果列被固定、辅助列被外部引用、结果列或条件列在求解模型中
/// 缺失、条件里出现指示列本身，都回退；本 writer 的独有门控是 **Big-M 冗余证明**（从 SDK 读条件列
/// 的 `LB` / `UB`，界不完整或松弛行不成立即回退）。绝不写语义不完整的关系；真正的 SDK 写入失败返回
/// `Err`，由调用方对整模型回退。
///
/// Serves [`InequalityStructure`] (`ospf-rust-core/src/symbol/functions/inequality.rs`), whose eager
/// expansion is "indicator = 1 ⇒ core relation; = 0 ⇒ the other core relation" plus two Big-M relaxed
/// rows; this writer writes the core relations through two `add_genconstr_indicator` calls and proves
/// the relaxed rows redundant first.
///
/// The remaining gates are shared with ABS/MAX/MIN/PWL: a fixed result column, externally referenced
/// helper columns, a result or condition column missing from the solve model, and the indicator column
/// appearing inside the condition itself all force a fallback. This writer's distinctive gate is the
/// **Big-M redundancy proof** (it reads the condition columns' `LB` / `UB` from the SDK and falls back
/// when the bounds are incomplete or a relaxed row does not hold). An incomplete relation is never
/// written, and a genuine SDK write failure returns `Err` so the caller can fall back for the whole
/// model.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiIndicatorWriter;

impl GurobiIndicatorWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiIndicatorWriter {
    fn name(&self) -> &str {
        "gurobi_indicator"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure
            .as_any()
            .downcast_ref::<InequalityStructure<f64>>()
            .is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        // 冗余证明要读 SDK 里的列属性，而刚建好的列在 Gurobi 的 lazy update 模式下仍是 pending
        // 对象（属性读取会因为对象尚未落地而失败），因此先把待定变更落地一次。`update()` 只推送尚未
        // 生效的变更，对之后的装行与求解没有语义影响。
        //
        // The redundancy proof reads column attributes from the SDK, but freshly created columns are
        // still pending objects under Gurobi's lazy update mode (an attribute read fails while the
        // object is not materialized), so the pending changes are flushed once first. `update()` only
        // pushes not-yet-applied changes and has no semantic effect on the later row loading and solve.
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_indicator writer failed to flush pending model changes before the big-M proof: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let structure = request
                .structure
                .as_any()
                .downcast_ref::<InequalityStructure<f64>>()
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "gurobi_indicator writer received a structure that is not a relation indicator"
                            .to_string(),
                    )
                })?;

            let plan = match plan_indicator_native(structure) {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 辅助列（`=` / `!=` 的 side 列）必须仅被本函数自己的关系行引用；本批允许的 kinds 没有
            // 辅助列，因此该门控在这里是恒真的一致性检查，保持与其它 writer 相同的拒绝语义。
            // The helper columns (the `=` / `!=` side column) must be referenced by this function's own
            // rows only; the kinds admitted here have no helper column, so this gate is a tautological
            // consistency check that keeps the same rejection semantics as the other writers.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let Some(indicator_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` indicator column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };

            let mut terms: Vec<(Var, f64)> = Vec::with_capacity(plan.coefficients.len());
            let mut missing_condition_column = false;
            for (index, coefficient) in &plan.coefficients {
                match container.variable_at(*index) {
                    Some(var) => terms.push((var, *coefficient)),
                    None => {
                        missing_condition_column = true;
                        break;
                    }
                }
            }
            if missing_condition_column {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` condition column is missing from the solve model",
                    plan.name
                ))));
                continue;
            }

            // 指示列不能同时出现在条件里：即时展开会给它额外的 Big-M 系数，而原生接口只把指示列当
            // 开关，二者不再是同一条关系。
            // The indicator column must not appear inside the condition: eager expansion gives it an
            // extra Big-M coefficient while the native interface only uses it as a switch, so the two
            // would no longer describe the same relation.
            if terms.iter().any(|(var, _)| *var == indicator_var) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` native lowering requires the indicator column to stay out of its own condition",
                    plan.name
                ))));
                continue;
            }

            // SDK 要求指示变量是二元变量；不是二元时保守回退，而不是把模型推向 SDK 报错后的整模型
            // 回退。
            // The SDK requires the indicator variable to be binary; anything else falls back
            // conservatively instead of pushing the model into an SDK error and a whole-model
            // fallback.
            let indicator_type = container
                .model_mut()
                .get_obj_attr(attr::VType, &indicator_var);
            match indicator_type {
                Ok(VarType::Binary) => {}
                Ok(other) => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "relation indicator `{}` native lowering requires a binary indicator column, got {other:?}",
                        plan.name
                    ))));
                    continue;
                }
                Err(error) => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "relation indicator `{}` native lowering could not read the indicator column type from the Gurobi model: {error}",
                        plan.name
                    ))));
                    continue;
                }
            }

            // Big-M 冗余证明：从列界算出 `s = Σ c_k x_k + constant` 在盒上的上下界。界不完整
            // （列无界 / 读不到）时无法证明等价性，保守回退而不是冒险写入。
            // Big-M redundancy proof: derive the box bounds of `s = Σ c_k x_k + constant` from the
            // column bounds. Incomplete bounds (unbounded or unreadable columns) mean equivalence
            // cannot be proven, so the writer conservatively falls back instead of writing on faith.
            let mut s_min = plan.constant;
            let mut s_max = plan.constant;
            let mut incomplete_bounds = None;
            for (var, coefficient) in &terms {
                let bounds = {
                    let model = container.model_mut();
                    match model.get_obj_attr(attr::LB, var) {
                        Ok(lb) => model
                            .get_obj_attr(attr::UB, var)
                            .map(|ub| (lb, ub))
                            .map_err(|error| error.to_string()),
                        Err(error) => Err(error.to_string()),
                    }
                };
                let (lower_bound, upper_bound) = match bounds {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        incomplete_bounds = Some(format!(
                            "could not read the condition column bounds from the Gurobi model: {error}"
                        ));
                        break;
                    }
                };
                // Gurobi 用 ±1e100（`grb::INFINITY`）表示无穷界，它本身是有限数，因此必须显式比较。
                // Gurobi represents infinite bounds as ±1e100 (`grb::INFINITY`), which are finite
                // numbers, so they must be compared explicitly.
                if !lower_bound.is_finite()
                    || !upper_bound.is_finite()
                    || lower_bound <= -INFINITY
                    || upper_bound >= INFINITY
                {
                    incomplete_bounds = Some(format!(
                        "condition column bounds [{lower_bound}, {upper_bound}] are not a finite interval"
                    ));
                    break;
                }
                if *coefficient >= 0.0 {
                    s_min += coefficient * lower_bound;
                    s_max += coefficient * upper_bound;
                } else {
                    s_min += coefficient * upper_bound;
                    s_max += coefficient * lower_bound;
                }
            }
            if let Some(reason) = incomplete_bounds {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }
            if let Err(reason) = plan.prove_big_m_relaxation(s_min, s_max) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "relation indicator `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 两条核心行分别写成一条指示约束：`ind = 1` 与 `ind = 0`。SDK 写入失败直接返回错误：
            // 调用方必须对整模型回退，而不是保留半写入状态。
            // The two core rows become one indicator constraint each: `ind = 1` and `ind = 0`. An SDK
            // write failure returns an error: the caller must fall back for the whole model instead of
            // keeping a half-written state.
            for (indicator_value, (relation, rhs), suffix) in [
                (true, plan.when_true, "indicator_true"),
                (false, plan.when_false, "indicator_false"),
            ] {
                let condition =
                    indicator_condition(&terms, plan.constant, relation, rhs);
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_{}", plan.name, suffix),
                        indicator_var,
                        indicator_value,
                        condition,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_indicator writer failed to write relation indicator `{}` ({suffix}): {error}",
                            plan.name
                        ))
                    })?;
            }

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_INDICATOR_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// IF-IN（离散值集合判定）原生写入的 schema 版本
/// Schema version of the native IF-IN (discrete set membership) write.
///
/// 与关系指示分开一个 writer 与 schema：原生结构不只包含指示约束，还包含一条把候选值指示列聚合起
/// 来的 `or` 一般约束，并且带一份**逐候选值**的 Big-M 冗余证明。把两者混在同一个 schema 下会让
/// 恢复阶段无法区分结构含义。
///
/// A separate writer and schema from the relation indicator: the native structure is not only a set of
/// indicator constraints but also an `or` general constraint aggregating the candidate indicators, plus
/// a **per-candidate** Big-M redundancy proof. Sharing one schema would make recovery unable to tell the
/// two meanings apart.
pub const GUROBI_IF_IN_SCHEMA: &str = "functions-if-in-1";

/// IF-IN Big-M 冗余证明允许的容差 / Tolerance allowed by the IF-IN Big-M redundancy proof.
///
/// 与关系指示的证明同源：取值 1e-9，比 `STRICT_BOUNDARY = 2e-8` 小一个多数量级，因此只用于吸收
/// 推断链（`max_difference + STRICT_BOUNDARY` 及其 ULP 扩张）上的浮点舍入，不会掩盖真正的界不足。
///
/// Same origin as the relation indicator's proof: 1e-9, more than an order of magnitude below
/// `STRICT_BOUNDARY = 2e-8`, so it only absorbs floating-point rounding along the inference chain
/// (`max_difference + STRICT_BOUNDARY` and its ULP expansion) and never hides a genuinely short bound.
pub const GUROBI_IF_IN_BIG_M_TOLERANCE: f64 = GUROBI_INDICATOR_BIG_M_TOLERANCE;

/// IF-IN 原生写入计划 / Plan for one native IF-IN write.
///
/// 每个候选值 `values[i]` 在平移量 `s_i = input - values[i]` 上贡献四条指示约束：
///
/// - `b_i = 1` ⇒ `s_i <= +tol` 与 `s_i >= -tol`（band，`tol` 取即时路径的 [`IF_IN_STEP_EPSILON`]）；
/// - `side_i = 0` ⇒ `s_i - M·b_i <= -STRICT_BOUNDARY`；
/// - `side_i = 1` ⇒ `s_i + M·b_i >= +STRICT_BOUNDARY`。
///
/// 两条 side 行里的 `M·b_i` 项让 `b_i = 1` 时该行退化成 Big-M 松弛，这与即时展开里 `out_lb` /
/// `out_ub` 只在 `(b_i, side_i) = (0, 1)` / `(0, 0)` 时才是核心行的结构逐项对应；`b_i = 1` 时即
/// 时展开的 out 行同样只剩松弛（见 [`Self::prove_big_m_relaxation`] 的证明条件）。
/// 结果列再由 `result = OR(b_0..b_{n-1})` 聚合，等价于即时展开的 `or_lb_*` / `or_ub` 两族行。
///
/// Every candidate `values[i]` contributes four indicator constraints on the shift
/// `s_i = input - values[i]`:
///
/// - `b_i = 1` ⇒ `s_i <= +tol` and `s_i >= -tol` (the band, with `tol` taken from the eager path's
///   [`IF_IN_STEP_EPSILON`]);
/// - `side_i = 0` ⇒ `s_i - M·b_i <= -STRICT_BOUNDARY`;
/// - `side_i = 1` ⇒ `s_i + M·b_i >= +STRICT_BOUNDARY`.
///
/// The `M·b_i` terms make those two side rows degenerate into Big-M relaxations when `b_i = 1`, which
/// mirrors the eager `out_lb` / `out_ub` rows being core rows only at `(b_i, side_i) = (0, 1)` / `(0, 0)`
/// (at `b_i = 1` the eager out rows are relaxations as well; see the proof conditions of
/// [`Self::prove_big_m_relaxation`]). The result column is then aggregated through
/// `result = OR(b_0..b_{n-1})`, equivalent to the eager `or_lb_*` / `or_ub` row families.
#[derive(Debug, Clone, PartialEq)]
pub struct IfInNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列（集合判定的二值列）/ Result column (the binary set-membership column)
    pub result: VariableId,
    /// 输入多项式的单项式：条件列下标 + 系数 / Input monomials: condition column index + coefficient
    pub coefficients: Vec<(usize, f64)>,
    /// 输入多项式的常数项 / Constant of the input polynomial
    pub input_constant: f64,
    /// 离散值集合，与指示列 / side 列一一对应 / Discrete values, aligned with the indicator and side columns
    pub values: Vec<f64>,
    /// 每个候选值的指示列 / Indicator column of every candidate
    pub indicator_columns: Vec<VariableId>,
    /// 每个候选值的 side 列 / Side column of every candidate
    pub side_columns: Vec<VariableId>,
    /// 结构创建时固定的 Big-M / Big-M fixed when the structure was created
    pub big_m: f64,
    /// band 容差（即时路径的同一常量 `STEP_EPSILON`）
    /// Band tolerance (the eager path's very `STEP_EPSILON`)
    pub tolerance: f64,
    /// 严格边界（即时路径的同一常量 `STRICT_BOUNDARY`）
    /// Strict boundary (the eager path's very `STRICT_BOUNDARY`)
    pub strict_boundary: f64,
}

impl IfInNativePlan {
    /// 证明第 `value_index` 个候选值在输入盒上被 Big-M 松弛掉的行恒成立。
    ///
    /// 需要成立的两条（其余松弛行都比它们弱）：
    ///
    /// - 下侧 `s_i >= strict_boundary - M`（`out_lb` 在 `(b_i, side_i) = (0, 0)`、`(1, 1)` 以及原生
    ///   side 行在 `b_i = 1` 时的形状）⇔ `M + s_i_min >= strict_boundary`；
    /// - 上侧 `s_i <= M - strict_boundary`（`out_ub` 在 `(b_i, side_i) = (0, 1)`、`(1, 0)` 以及原生
    ///   side 行在 `b_i = 1` 时的形状）⇔ `M - s_i_max >= strict_boundary`。
    ///
    /// band 行的松弛只要求 `M - s_i_max >= -tol` 与 `M + s_i_min >= -tol`，因为
    /// `strict_boundary = 2·tol > -tol`，上面两条一旦成立就蕴含它们。
    ///
    /// Prove that the rows relaxed through the Big-M hold for candidate `value_index` on the input box.
    ///
    /// Two conditions suffice (every other relaxed row is weaker):
    ///
    /// - the lower side `s_i >= strict_boundary - M` (the shape of `out_lb` at
    ///   `(b_i, side_i) = (0, 0)`, `(1, 1)` and of the native side row at `b_i = 1`) ⇔
    ///   `M + s_i_min >= strict_boundary`;
    /// - the upper side `s_i <= M - strict_boundary` (the shape of `out_ub` at
    ///   `(b_i, side_i) = (0, 1)`, `(1, 0)` and of the native side row at `b_i = 1`) ⇔
    ///   `M - s_i_max >= strict_boundary`.
    ///
    /// The band relaxations only need `M - s_i_max >= -tol` and `M + s_i_min >= -tol`, and since
    /// `strict_boundary = 2·tol > -tol` the two conditions above imply them.
    pub fn prove_big_m_relaxation(
        &self,
        value_index: usize,
        s_min: f64,
        s_max: f64,
    ) -> std::result::Result<(), String> {
        if !s_min.is_finite() || !s_max.is_finite() || s_min > s_max {
            return Err(format!(
                "if_in `{}` candidate {value_index} has no finite domain, got [{s_min}, {s_max}]",
                self.name
            ));
        }
        if self.big_m + s_min < self.strict_boundary - GUROBI_IF_IN_BIG_M_TOLERANCE {
            return Err(format!(
                "if_in `{}` big-M {} does not imply the eager lower relaxation `s >= {} - M` for candidate {value_index} on [{s_min}, {s_max}]",
                self.name, self.big_m, self.strict_boundary
            ));
        }
        if self.big_m - s_max < self.strict_boundary - GUROBI_IF_IN_BIG_M_TOLERANCE {
            return Err(format!(
                "if_in `{}` big-M {} does not imply the eager upper relaxation `s <= M - {}` for candidate {value_index} on [{s_min}, {s_max}]",
                self.name, self.big_m, self.strict_boundary
            ));
        }
        Ok(())
    }

    /// 第 `value_index` 个候选值在输入盒 `[input_min, input_max]` 上的平移量范围
    /// The shift range of candidate `value_index` over the input box `[input_min, input_max]`
    pub fn shifted_bounds(
        &self,
        input_min: f64,
        input_max: f64,
        value_index: usize,
    ) -> (f64, f64) {
        let value = self.values[value_index];
        (input_min - value, input_max - value)
    }
}

/// 判断一个 IF-IN 结构是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 允许的形态：值集合非空、Big-M 正且有限、输入多项式至少有一个非零变量项、每个集合值与输入常数
/// 都有限、辅助列数量与集合大小一致。核心关系与两个容差直接来自符号文件里的
/// [`if_in_value_core_relations`]，因此 band 容差与严格边界与即时路径逐位相同。
///
/// 明确拒绝（回退 EAGER）：
///
/// - **空值集合**：即时展开退化成单条 `result = 0`，而 `or` 一般约束没有零个操作数的含义；
/// - **输入没有变量项**：指示约束的线性表达式会变空，SDK 无法表达；
/// - **辅助列与集合大小不一致**：说明结构不是本 writer 认识的那一种，宁可回退。
///
/// **本函数不做盒证明**：逐候选值的 Big-M 冗余需要读 SDK 列界，由 writer 在解析出列之后完成
/// （见 [`GurobiIfInWriter::write_batch`]）。
///
/// Decide whether an IF-IN structure may be written natively and produce its write plan (SDK-free).
///
/// Admitted shape: a non-empty value set, a positive finite Big-M, an input polynomial with at least one
/// non-zero variable term, finite set values and input constant, and a helper count matching the set
/// size. The core relations and both tolerances come straight from the symbol file's
/// [`if_in_value_core_relations`], so the band tolerance and strict boundary are bit-identical to the
/// eager path's.
///
/// Explicitly rejected (kept on EAGER):
///
/// - **an empty value set**: eager expansion collapses to the single row `result = 0`, while an `or`
///   general constraint has no meaning with zero operands;
/// - **an input without variable terms**: the indicator constraints' linear expression would be empty and
///   the SDK cannot express it;
/// - **a helper count that does not match the set size**: the structure is not the one this writer knows,
///   and a fallback is safer than guessing.
///
/// This function performs **no box proof**: the per-candidate Big-M redundancy needs the SDK column bounds
/// and therefore happens in the writer once the columns are resolved (see
/// [`GurobiIfInWriter::write_batch`]).
pub fn plan_if_in_native(
    structure: &IfInStructure<f64>,
) -> std::result::Result<IfInNativePlan, FallbackReason> {
    let name = structure.name();

    let raw_values = structure.values();
    if raw_values.is_empty() {
        return Err(FallbackReason::Rejected(format!(
            "if_in `{name}` native lowering requires at least one candidate value: the eager expansion collapses to `result = 0` while an `or` general constraint has no zero-operand meaning"
        )));
    }
    let mut values = Vec::with_capacity(raw_values.len());
    for (index, value) in raw_values.iter().enumerate() {
        if !value.is_finite() {
            return Err(FallbackReason::Rejected(format!(
                "if_in `{name}` native lowering requires finite candidate values, got {value} at index {index}"
            )));
        }
        values.push(*value);
    }

    let big_m = structure.big_m();
    if !big_m.is_finite() || big_m <= 0.0 {
        return Err(FallbackReason::Rejected(format!(
            "if_in `{name}` native lowering requires a positive finite big-M, got {big_m}"
        )));
    }

    let input = structure.input_polynomial();
    let input_constant = *input.constant_term();
    if !input_constant.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "if_in `{name}` native lowering requires a finite input constant, got {input_constant}"
        )));
    }
    let mut coefficients = Vec::with_capacity(input.monomials().len());
    for monomial in input.monomials() {
        let coefficient = *monomial.coefficient();
        if !coefficient.is_finite() {
            return Err(FallbackReason::Rejected(format!(
                "if_in `{name}` native lowering requires finite input coefficients, got {coefficient}"
            )));
        }
        coefficients.push((monomial.var_index(), coefficient));
    }
    if !coefficients
        .iter()
        .any(|(_, coefficient)| *coefficient != 0.0)
    {
        return Err(FallbackReason::Rejected(format!(
            "if_in `{name}` native lowering requires at least one variable term in the input, got {} monomials with no non-zero coefficient",
            coefficients.len()
        )));
    }

    let indicator_columns = structure.indicator_columns();
    let side_columns = structure.side_columns();
    if indicator_columns.len() != values.len() || side_columns.len() != values.len() {
        return Err(FallbackReason::Rejected(format!(
            "if_in `{name}` native lowering requires one indicator and one side column per candidate value, got {} indicators and {} side columns for {} values",
            indicator_columns.len(),
            side_columns.len(),
            values.len()
        )));
    }

    let core = if_in_value_core_relations();
    Ok(IfInNativePlan {
        name: name.to_string(),
        result: structure.result().clone(),
        coefficients,
        input_constant,
        values,
        indicator_columns,
        side_columns,
        big_m,
        // 两个容差都取即时路径的同一常量，本模块不新增任何自造容差。
        // Both tolerances are the eager path's very constants; this module invents none.
        tolerance: core.band_tolerance,
        strict_boundary: core.strict_boundary,
    })
}

/// Gurobi 的 IF-IN（离散值集合判定）原生 writer / Gurobi's native IF-IN (set membership) writer.
///
/// 服务 [`IfInStructure`]（`ospf-rust-core/src/symbol/functions/if_in.rs`）：每个候选值写四条
/// `add_genconstr_indicator`（band 两条 + side 两条，见 [`IfInNativePlan`]），再用一条
/// `add_genconstr_or` 聚合候选值指示列，与即时展开的 `or_lb_*` / `or_ub` 行族等价（二元变量上
/// `result = OR(b_i)` 正是那两族行的精确 hull）。
///
/// 门控与既有 writer 一致：结果列被固定、辅助列（每个候选值的指示列与 side 列）被外部引用、结果列
/// 或条件列在求解模型中缺失、条件里出现指示列 / side 列 / 结果列、指示列与 side 列不是二元，都
/// 回退；本 writer 的独有门控是**逐候选值的 Big-M 冗余证明**（从 SDK 读条件列界，算输入盒，再按
/// 候选值平移；界不完整或某条松弛行不成立即回退）。绝不写语义不完整的关系；真正的 SDK 写入失败
/// 返回 `Err`，由调用方对整模型回退。
///
/// Serves [`IfInStructure`] (`ospf-rust-core/src/symbol/functions/if_in.rs`): every candidate writes
/// four `add_genconstr_indicator` calls (two band rows plus two side rows, see [`IfInNativePlan`]) and one
/// `add_genconstr_or` aggregates the candidate indicators, equivalent to the eager `or_lb_*` / `or_ub`
/// row families (over binary variables `result = OR(b_i)` is exactly their precise hull).
///
/// The gates match the existing writers: a fixed result column, externally referenced helper columns
/// (every candidate's indicator and side column), a result or condition column missing from the solve
/// model, a condition containing an indicator / side / result column, and a non-binary indicator or side
/// column all force a fallback. This writer's distinctive gate is the **per-candidate Big-M redundancy
/// proof** (it reads the condition column bounds from the SDK, forms the input box and shifts it per
/// candidate; incomplete bounds or any relaxed row that does not hold forces a fallback). An incomplete
/// relation is never written, and a genuine SDK write failure returns `Err` so the caller can fall back
/// for the whole model.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiIfInWriter;

impl GurobiIfInWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiIfInWriter {
    fn name(&self) -> &str {
        "gurobi_if_in"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure
            .as_any()
            .downcast_ref::<IfInStructure<f64>>()
            .is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        // 与 PWL / 关系指示同理：冗余证明要读列属性，先落地一次待定变更。
        // As with PWL and the relation indicator: the redundancy proof reads column attributes, so the
        // pending changes are flushed once first.
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_if_in writer failed to flush pending model changes before the big-M proof: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let structure = request
                .structure
                .as_any()
                .downcast_ref::<IfInStructure<f64>>()
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "gurobi_if_in writer received a structure that is not an if-in structure"
                            .to_string(),
                    )
                })?;

            let plan = match plan_if_in_native(structure) {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 候选值的指示列与 side 列都只被本函数自己的行引用时才允许原生写入：`or` 一般约束与四条
            // 指示约束虽然仍然约束这些列，但一旦模型在别处引用它们，原生写入与即时展开在这些列上的
            // 含义就可能被外部行区分出来。
            // A native write is allowed only while every candidate's indicator and side column is
            // referenced by this function's own rows: the `or` general constraint and the four indicator
            // constraints still constrain those columns, but once the model references them elsewhere an
            // external row could tell the native write and eager expansion apart on them.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let Some(result_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` result column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };

            let mut indicator_vars = Vec::with_capacity(plan.indicator_columns.len());
            let mut side_vars = Vec::with_capacity(plan.side_columns.len());
            for (indicator, side) in plan.indicator_columns.iter().zip(plan.side_columns.iter()) {
                match (container.variable(indicator), container.variable(side)) {
                    (Some(indicator_var), Some(side_var)) => {
                        indicator_vars.push(indicator_var);
                        side_vars.push(side_var);
                    }
                    _ => break,
                }
            }
            if indicator_vars.len() != plan.values.len() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` candidate indicator or side column is missing from the solve model",
                    plan.name
                ))));
                continue;
            }

            let mut condition_terms: Vec<(Var, f64)> = Vec::with_capacity(plan.coefficients.len());
            let mut missing_condition_column = false;
            for (index, coefficient) in &plan.coefficients {
                match container.variable_at(*index) {
                    Some(var) => condition_terms.push((var, *coefficient)),
                    None => {
                        missing_condition_column = true;
                        break;
                    }
                }
            }
            if missing_condition_column {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` condition column is missing from the solve model",
                    plan.name
                ))));
                continue;
            }

            // 条件列不能与结果列或任何指示列 / side 列重合：指示列不得出现在它自己的约束里，
            // side 行的 `M·b_i` 项也要求 `b_i` 是独立的列。
            // A condition column must not coincide with the result or any indicator / side column: an
            // indicator variable may not appear inside its own constraint, and the side rows' `M·b_i`
            // term requires `b_i` to be a separate column.
            if condition_terms.iter().any(|(var, _)| {
                *var == result_var || indicator_vars.contains(var) || side_vars.contains(var)
            }) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering requires the condition columns to stay clear of the result, indicator and side columns",
                    plan.name
                ))));
                continue;
            }

            // SDK 要求指示变量与 `or` 的结果/操作数都是二元变量。
            // The SDK requires the indicator variables and the `or` result/operands to be binary.
            let mut binary_variables = Vec::with_capacity(1 + indicator_vars.len() + side_vars.len());
            binary_variables.push(result_var);
            binary_variables.extend(indicator_vars.iter().copied());
            binary_variables.extend(side_vars.iter().copied());
            let mut non_binary = None;
            for var in &binary_variables {
                match container.model_mut().get_obj_attr(attr::VType, var) {
                    Ok(VarType::Binary) => {}
                    Ok(other) => {
                        non_binary = Some(format!("{other:?}"));
                        break;
                    }
                    Err(error) => {
                        non_binary = Some(error.to_string());
                        break;
                    }
                }
            }
            if let Some(reason) = non_binary {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering requires the result, indicator and side columns to be binary: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 逐候选值的 Big-M 冗余证明：先从列界算出输入盒 `[input_min, input_max]`，再按候选值平移。
            // Per-candidate Big-M redundancy proof: first derive the input box `[input_min, input_max]`
            // from the column bounds, then shift it per candidate.
            let mut input_min = plan.input_constant;
            let mut input_max = plan.input_constant;
            let mut incomplete_bounds = None;
            for (var, coefficient) in &condition_terms {
                let bounds = {
                    let model = container.model_mut();
                    match model.get_obj_attr(attr::LB, var) {
                        Ok(lb) => model
                            .get_obj_attr(attr::UB, var)
                            .map(|ub| (lb, ub))
                            .map_err(|error| error.to_string()),
                        Err(error) => Err(error.to_string()),
                    }
                };
                let (lower_bound, upper_bound) = match bounds {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        incomplete_bounds = Some(format!(
                            "could not read the condition column bounds from the Gurobi model: {error}"
                        ));
                        break;
                    }
                };
                // Gurobi 用 ±1e100（`grb::INFINITY`）表示无穷界，它本身是有限数，因此必须显式比较。
                // Gurobi represents infinite bounds as ±1e100 (`grb::INFINITY`), which are finite
                // numbers, so they must be compared explicitly.
                if !lower_bound.is_finite()
                    || !upper_bound.is_finite()
                    || lower_bound <= -INFINITY
                    || upper_bound >= INFINITY
                {
                    incomplete_bounds = Some(format!(
                        "condition column bounds [{lower_bound}, {upper_bound}] are not a finite interval"
                    ));
                    break;
                }
                if *coefficient >= 0.0 {
                    input_min += coefficient * lower_bound;
                    input_max += coefficient * upper_bound;
                } else {
                    input_min += coefficient * upper_bound;
                    input_max += coefficient * lower_bound;
                }
            }
            if let Some(reason) = incomplete_bounds {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }

            let mut failed_proof = None;
            for value_index in 0..plan.values.len() {
                let (s_min, s_max) = plan.shifted_bounds(input_min, input_max, value_index);
                if let Err(reason) = plan.prove_big_m_relaxation(value_index, s_min, s_max) {
                    failed_proof = Some(reason);
                    break;
                }
            }
            if let Some(reason) = failed_proof {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "if_in `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 每个候选值四条指示约束，最后一条 `or` 聚合候选值指示列。
            // Four indicator constraints per candidate value, then one `or` aggregating the candidate
            // indicators. An SDK write failure returns an error so the caller falls back for the whole
            // model instead of keeping a half-written state.
            for value_index in 0..plan.values.len() {
                let shifted_constant = plan.input_constant - plan.values[value_index];
                let indicator_var = indicator_vars[value_index];
                let side_var = side_vars[value_index];

                let band_ub = indicator_condition(
                    &condition_terms,
                    shifted_constant,
                    ConstraintRelation::LessEqual,
                    plan.tolerance,
                );
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_pt{value_index}_band_ub", plan.name),
                        indicator_var,
                        true,
                        band_ub,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_if_in writer failed to write if_in `{}` band upper row for candidate {value_index}: {error}",
                            plan.name
                        ))
                    })?;

                let band_lb = indicator_condition(
                    &condition_terms,
                    shifted_constant,
                    ConstraintRelation::GreaterEqual,
                    -plan.tolerance,
                );
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_pt{value_index}_band_lb", plan.name),
                        indicator_var,
                        true,
                        band_lb,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_if_in writer failed to write if_in `{}` band lower row for candidate {value_index}: {error}",
                            plan.name
                        ))
                    })?;

                let mut out_ub_terms = condition_terms.clone();
                out_ub_terms.push((indicator_var, -plan.big_m));
                let out_ub = indicator_condition(
                    &out_ub_terms,
                    shifted_constant,
                    ConstraintRelation::LessEqual,
                    -plan.strict_boundary,
                );
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_pt{value_index}_out_ub", plan.name),
                        side_var,
                        false,
                        out_ub,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_if_in writer failed to write if_in `{}` out upper row for candidate {value_index}: {error}",
                            plan.name
                        ))
                    })?;

                let mut out_lb_terms = condition_terms.clone();
                out_lb_terms.push((indicator_var, plan.big_m));
                let out_lb = indicator_condition(
                    &out_lb_terms,
                    shifted_constant,
                    ConstraintRelation::GreaterEqual,
                    plan.strict_boundary,
                );
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_pt{value_index}_out_lb", plan.name),
                        side_var,
                        true,
                        out_lb,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_if_in writer failed to write if_in `{}` out lower row for candidate {value_index}: {error}",
                            plan.name
                        ))
                    })?;
            }

            container
                .model_mut()
                .add_genconstr_or(
                    &format!("{}_or", plan.name),
                    result_var,
                    indicator_vars.iter().copied(),
                )
                .map_err(|error| {
                    ModelError::InvalidConstraint(format!(
                        "gurobi_if_in writer failed to write if_in `{}` or constraint: {error}",
                        plan.name
                    ))
                })?;

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_IF_IN_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// AND 原生写入的 schema 版本 / Schema version of the native AND write.
///
/// schema 变化表示原生结构的含义变化，恢复阶段必须拒绝旧指纹。
/// A schema change means the native structure's meaning changed and recovery must reject older
/// fingerprints.
pub const GUROBI_AND_SCHEMA: &str = "functions-and-1";

/// OR 原生写入的 schema 版本 / Schema version of the native OR write.
pub const GUROBI_OR_SCHEMA: &str = "functions-or-1";

/// 逻辑（AND/OR）原生写入计划 / Plan for one native logical (AND/OR) write.
///
/// 紧凑 hull 的两族行只有两种形状：
///
/// - AND：`result - input_i <= 0`（每个操作数一条）与 `sum(input) - result <= n - 1`；
/// - OR：`result - input_i >= 0`（每个操作数一条）与 `sum(input) - result >= 0`。
///
/// 在二元变量上这两族行合起来正是 `result = AND(inputs)` / `result = OR(inputs)` 的**精确两侧
/// hull**，与 SDK 的 `add_genconstr_and` / `add_genconstr_or`（对二元变量实现的就是同一关系）逐点
/// 等价。计划里没有 Big-M，也没有辅助列：紧凑 hull 不依赖 M，操作数就是模型既有列。
///
/// The compact hull's two row families have only two shapes:
///
/// - AND: `result - input_i <= 0` (one per operand) plus `sum(input) - result <= n - 1`;
/// - OR: `result - input_i >= 0` (one per operand) plus `sum(input) - result >= 0`.
///
/// Over binary variables those families are exactly the **precise two-sided hull** of
/// `result = AND(inputs)` / `result = OR(inputs)`, hence pointwise equivalent to the SDK's
/// `add_genconstr_and` / `add_genconstr_or` (which implement the same relation for binary variables).
/// The plan carries no Big-M and no helper column: the compact hull does not depend on M and its
/// operands are existing model columns.
#[derive(Debug, Clone, PartialEq)]
pub struct LogicalNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列 / Result column
    pub result: VariableId,
    /// 操作数列在列视图中的位置 / Positions of the operand columns in the column view
    pub operand_indices: Vec<usize>,
    /// 是否为 AND（`false` 表示 OR）/ Whether this is an AND (`false` means OR)
    pub conjunction: bool,
}

/// 规划一次逻辑原生写入 / Plan one native logical write.
fn plan_logical(
    kind: &str,
    name: &str,
    result: &VariableId,
    operand_indices: Option<Vec<usize>>,
    conjunction: bool,
) -> std::result::Result<LogicalNativePlan, FallbackReason> {
    // 操作数不是"系数 1、常数 0 的单项式"时，即时展开走的不是紧凑 hull 分支（那需要非零指示列与
    // side 列），原生 and/or 表达不了那些行，因此明确拒绝并回退。
    // When an operand is not a "coefficient 1, zero constant" monomial, eager expansion does not take
    // the compact hull branch (that needs non-zero indicator and side columns), and the native and/or
    // constraint cannot express those rows; the structure is therefore rejected and falls back.
    let Some(operand_indices) = operand_indices else {
        return Err(FallbackReason::Rejected(format!(
            "{kind} `{name}` native lowering requires every operand to be a direct binary variable (a single unit-coefficient monomial with zero constant): the compact hull is the only form the native relation expresses"
        )));
    };
    if operand_indices.is_empty() {
        return Err(FallbackReason::Rejected(format!(
            "{kind} `{name}` native lowering requires at least one operand"
        )));
    }
    Ok(LogicalNativePlan {
        name: name.to_string(),
        result: result.clone(),
        operand_indices,
        conjunction,
    })
}

/// 规划 AND 的原生写入 / Plan the native write of an AND structure.
pub fn plan_and_native(
    structure: &AndStructure<f64>,
) -> std::result::Result<LogicalNativePlan, FallbackReason> {
    plan_logical(
        "and",
        structure.name(),
        structure.result(),
        structure.operand_indices(),
        true,
    )
}

/// 规划 OR 的原生写入 / Plan the native write of an OR structure.
pub fn plan_or_native(
    structure: &OrStructure<f64>,
) -> std::result::Result<LogicalNativePlan, FallbackReason> {
    plan_logical(
        "or",
        structure.name(),
        structure.result(),
        structure.operand_indices(),
        false,
    )
}

/// Gurobi 的逻辑（AND/OR）原生 writer / Gurobi's native logical (AND/OR) writer.
///
/// 服务 [`AndStructure`] / [`OrStructure`]（`ospf-rust-core/src/symbol/functions/and.rs`）：它们的
/// 延迟结构**只在所有输入都是直接二值变量时**才被暴露（`deferred_structure_with_tokens` 的准入），
/// 此时即时展开提前返回紧凑 hull 的两族行，且结构不上报任何辅助列。因此本 writer 只需一条
/// `add_genconstr_and` / `add_genconstr_or` 就与即时展开逐点等价，**不需要 Big-M 冗余证明**（hull
/// 不依赖 M）。
///
/// 门控：结果列被固定、结果列出现在操作数里（自引用）、结果列或任一操作数列在求解模型中缺失、
/// 结果列或任一操作数列不是二元，都回退。辅助列门控保留为恒真的一致性检查（结构上报的 helpers 为空）。
/// 真正的 SDK 写入失败返回 `Err`，由调用方对整模型回退。
///
/// 约束命名用 `{name}_and_native` / `{name}_or_native`，与 IF-IN 用来聚合候选值指示列的那条
/// `{name}_or` 明确区分：两者虽然都落在 `add_genconstr_or` 上，但语义位置不同（一个是函数的定义
/// 关系，一个是候选值聚合），各自独立写入、不共用约束名。
///
/// Serves [`AndStructure`] / [`OrStructure`] (`ospf-rust-core/src/symbol/functions/and.rs`): their
/// deferred structures are exposed **only when every input is a direct binary variable** (the admission
/// rule of `deferred_structure_with_tokens`), in which case eager expansion returns early with the
/// compact hull's two row families and the structure reports no helper column. One
/// `add_genconstr_and` / `add_genconstr_or` therefore matches eager expansion pointwise and **no Big-M
/// redundancy proof is needed** (the hull does not depend on M).
///
/// Gates: a fixed result column, the result column appearing among the operands (self-reference), a
/// result or operand column missing from the solve model, and a non-binary result or operand column all
/// force a fallback. The helper gate is kept as a tautological consistency check (the structure reports
/// no helpers). A genuine SDK write failure returns `Err` so the caller can fall back for the whole
/// model.
///
/// Constraints are named `{name}_and_native` / `{name}_or_native`, explicitly distinct from the
/// `{name}_or` IF-IN uses to aggregate candidate indicators: both land on `add_genconstr_or` but in
/// different semantic positions (a function's defining relation versus a candidate aggregation), so they
/// are written independently and never share a constraint name.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiLogicalWriter {
    conjunction: bool,
}

impl GurobiLogicalWriter {
    /// 创建 AND writer / Create the AND writer.
    pub fn conjunction() -> Self {
        Self { conjunction: true }
    }

    /// 创建 OR writer / Create the OR writer.
    pub fn disjunction() -> Self {
        Self { conjunction: false }
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiLogicalWriter {
    fn name(&self) -> &str {
        if self.conjunction {
            "gurobi_and"
        } else {
            "gurobi_or"
        }
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        if self.conjunction {
            structure
                .as_any()
                .downcast_ref::<AndStructure<f64>>()
                .is_some()
        } else {
            structure
                .as_any()
                .downcast_ref::<OrStructure<f64>>()
                .is_some()
        }
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        let kind = if self.conjunction { "and" } else { "or" };
        let schema = if self.conjunction {
            GUROBI_AND_SCHEMA
        } else {
            GUROBI_OR_SCHEMA
        };

        // 二元类型检查要读列属性，先落地一次待定变更（与 PWL / 指示 writer 同理）。
        // The binary type check reads column attributes, so the pending changes are flushed once first
        // (as in the PWL and indicator writers).
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_{kind} writer failed to flush pending model changes before the type check: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let plan = if self.conjunction {
                let structure = request
                    .structure
                    .as_any()
                    .downcast_ref::<AndStructure<f64>>()
                    .ok_or_else(|| {
                        ModelError::InvalidConstraint(
                            "gurobi_and writer received a structure that is not an AND structure"
                                .to_string(),
                        )
                    })?;
                plan_and_native(structure)
            } else {
                let structure = request
                    .structure
                    .as_any()
                    .downcast_ref::<OrStructure<f64>>()
                    .ok_or_else(|| {
                        ModelError::InvalidConstraint(
                            "gurobi_or writer received a structure that is not an OR structure"
                                .to_string(),
                        )
                    })?;
                plan_or_native(structure)
            };

            let plan = match plan {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 紧凑 hull 不上报辅助列（操作数是模型既有列），因此本门控恒真；保留下它与其它 writer 保持
            // 同一套拒绝语义。
            // The compact hull reports no helper columns (its operands are existing model columns), so
            // this gate is tautological; it is kept to preserve the same rejection semantics as the
            // other writers.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let Some(result_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` result column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };

            let mut operand_vars = Vec::with_capacity(plan.operand_indices.len());
            let mut missing_operand = false;
            for index in &plan.operand_indices {
                match container.variable_at(*index) {
                    Some(var) => operand_vars.push(var),
                    None => {
                        missing_operand = true;
                        break;
                    }
                }
            }
            if missing_operand {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` operand column is missing from the solve model",
                    plan.name
                ))));
                continue;
            }

            // 结果列不得作为操作数：即时展开的 `result = AND(result)` 退化关系与 SDK 一般约束的自引用
            // 用法不是同一件事。
            // The result column must not be an operand: the degenerate eager relation
            // `result = AND(result)` is not the same thing as a self-referencing SDK general constraint.
            if operand_vars.contains(&result_var) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` native lowering requires operand columns distinct from the result",
                    plan.name
                ))));
                continue;
            }

            // SDK 要求 `result = AND/OR(operands)` 的所有列都是二元变量。
            // The SDK requires every column of `result = AND/OR(operands)` to be binary.
            let mut binary_variables = Vec::with_capacity(1 + operand_vars.len());
            binary_variables.push(result_var);
            binary_variables.extend(operand_vars.iter().copied());
            let mut non_binary = None;
            for var in &binary_variables {
                match container.model_mut().get_obj_attr(attr::VType, var) {
                    Ok(VarType::Binary) => {}
                    Ok(other) => {
                        non_binary = Some(format!("{other:?}"));
                        break;
                    }
                    Err(error) => {
                        non_binary = Some(error.to_string());
                        break;
                    }
                }
            }
            if let Some(reason) = non_binary {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "{kind} `{}` native lowering requires the result and operand columns to be binary: {reason}",
                    plan.name
                ))));
                continue;
            }

            // SDK 写入失败直接返回错误：调用方必须对整模型回退，而不是保留半写入状态。
            // An SDK write failure returns an error: the caller must fall back for the whole model
            // instead of keeping a half-written state.
            let write = if self.conjunction {
                container.model_mut().add_genconstr_and(
                    &format!("{}_and_native", plan.name),
                    result_var,
                    operand_vars.iter().copied(),
                )
            } else {
                container.model_mut().add_genconstr_or(
                    &format!("{}_or_native", plan.name),
                    result_var,
                    operand_vars.iter().copied(),
                )
            };
            write.map_err(|error| {
                ModelError::InvalidConstraint(format!(
                    "gurobi_{kind} writer failed to write {kind} `{}`: {error}",
                    plan.name
                ))
            })?;

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                schema,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// 二值化原生写入的 schema 版本 / Schema version of the native binaryzation write.
///
/// 与关系指示分开一个 writer 与 schema：二值化的即时展开是「阈值两侧各一条单边关系」（取真一侧
/// `s >= ε_low`、取假一侧 `s <= -ε_up`），严格性常量也是另一个（`16·f64::EPSILON`），把两者混在一个
/// schema 下会让恢复阶段无法区分结构含义。
///
/// A separate writer and schema from the relation indicator: binaryzation's eager expansion is "one
/// one-sided relation per threshold side" (true side `s >= ε_low`, false side `s <= -ε_up`) and its
/// strictness constant is a different one (`16·f64::EPSILON`), so sharing a schema would make recovery
/// unable to tell the two meanings apart.
pub const GUROBI_BINARYZATION_SCHEMA: &str = "functions-binaryzation-1";

/// 二值化 Big-M 冗余证明允许的容差 / Tolerance allowed by the binaryzation Big-M redundancy proof.
///
/// 与关系指示的证明同源：1e-9，比即时展开的严格边界 `16·f64::EPSILON ≈ 3.55e-15` 大六个数量级，
/// 因此只吸收推断链（`max(|input - threshold| 盒界, 1)`）上的浮点舍入，不会掩盖真正的界不足。
///
/// Same origin as the relation indicator's proof: 1e-9, six orders of magnitude above the eager strict
/// boundary `16·f64::EPSILON ≈ 3.55e-15`, so it only absorbs floating-point rounding along the inference
/// chain (`max(|input - threshold| box bound, 1)`) and never hides a genuinely short bound.
pub const GUROBI_BINARYZATION_BIG_M_TOLERANCE: f64 = GUROBI_INDICATOR_BIG_M_TOLERANCE;

/// 二值化原生写入计划 / Plan for one native binaryzation write.
///
/// 即时展开只有两行（`s - M·y ≥ lower_rhs` 与 `s - M·y ≤ upper_rhs`，`s = input - threshold`），把指示列
/// 固定为 1 / 0 后，每行对 `s` 的投影恰好是「取真一侧」与「取假一侧」的单边关系，因此原生写入就是
/// 两条 `add_genconstr_indicator`（`ind_val = true` / `false`）。另一对投影只剩 Big-M 松弛，由
/// [`Self::prove_big_m_relaxation`] 单独证明。核心关系与两个松弛右端直接来自符号文件里的
/// [`binaryzation_core_relations`]，因此严格性 ε 与即时路径**逐位相同**。
///
/// The eager expansion has exactly two rows (`s - M·y ≥ lower_rhs` and `s - M·y ≤ upper_rhs` with
/// `s = input - threshold`); fixing the indicator column to 1 / 0 projects each row onto `s` as one
/// one-sided relation for the true side and one for the false side, so the native write is exactly two
/// `add_genconstr_indicator` calls (`ind_val = true` / `false`). The other pair of projections reduces to
/// Big-M relaxations proven separately by [`Self::prove_big_m_relaxation`]. The core relations and both
/// relaxation right-hand sides come straight from the symbol file's [`binaryzation_core_relations`], so the
/// strictness ε is **bit-identical** to the eager path's.
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryzationNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 指示列 `y`，同时是本函数的**结果列** / Indicator column `y`, which is also the result column
    pub result: VariableId,
    /// 平移后线性式 `s` 的单项式：条件列下标 + 系数
    /// Monomials of the shifted form `s`: condition column index + coefficient
    pub coefficients: Vec<(usize, f64)>,
    /// 平移后线性式的常数项 `input_constant - threshold`
    /// Constant of the shifted form: `input_constant - threshold`
    pub constant: f64,
    /// 结构创建时固定的 Big-M / Big-M fixed when the structure was created
    pub big_m: f64,
    /// 指示列取真时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 1`
    pub when_true: (ConstraintRelation, f64),
    /// 指示列取假时的核心关系 `s REL rhs` / Core relation `s REL rhs` for `indicator = 0`
    pub when_false: (ConstraintRelation, f64),
    /// 需要证明的下侧松弛 `s >= relaxed_lower_rhs - M`
    /// Lower relaxation to prove: `s >= relaxed_lower_rhs - M`
    pub relaxed_lower_rhs: f64,
    /// 需要证明的上侧松弛 `s <= M - relaxed_upper_rhs`
    /// Upper relaxation to prove: `s <= M - relaxed_upper_rhs`
    pub relaxed_upper_rhs: f64,
}

impl BinaryzationNativePlan {
    /// 证明即时展开里被 Big-M 松弛掉的两条行在输入盒 `[s_min, s_max]` 上恒成立。
    ///
    /// 只要两条松弛行在整个盒上成立，即时展开相对原生两条指示约束就没有额外约束，两者的可行解集合
    /// 一致（差异仅在证明容差量级）。
    ///
    /// Prove that both rows eager expansion relaxes through the Big-M hold on the input box
    /// `[s_min, s_max]`.
    ///
    /// As long as both relaxed rows hold on the whole box, eager expansion adds no constraint beyond the
    /// native pair of indicator constraints and the two feasible sets coincide (up to the proof's
    /// tolerance).
    pub fn prove_big_m_relaxation(
        &self,
        s_min: f64,
        s_max: f64,
    ) -> std::result::Result<(), String> {
        if !s_min.is_finite() || !s_max.is_finite() || s_min > s_max {
            return Err(format!(
                "binaryzation `{}` has no finite input domain, got [{s_min}, {s_max}]",
                self.name
            ));
        }
        if self.big_m + s_min < self.relaxed_lower_rhs - GUROBI_BINARYZATION_BIG_M_TOLERANCE {
            return Err(format!(
                "binaryzation `{}` big-M {} does not imply the eager lower relaxation `s >= {} - M` on [{s_min}, {s_max}]",
                self.name, self.big_m, self.relaxed_lower_rhs
            ));
        }
        if self.big_m - s_max < self.relaxed_upper_rhs - GUROBI_BINARYZATION_BIG_M_TOLERANCE {
            return Err(format!(
                "binaryzation `{}` big-M {} does not imply the eager upper relaxation `s <= M - {}` on [{s_min}, {s_max}]",
                self.name, self.big_m, self.relaxed_upper_rhs
            ));
        }
        Ok(())
    }
}

/// 判断一个二值化结构是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 允许的形态：输入多项式是任意仿射式（`add_genconstr_indicator` 的条件是**一般线性表达式**，不像
/// ABS/极值那样只接受列，因此**不需要**桥接列，也不会改变公开列），至少有一个非零变量项，阈值、
/// 输入常数、各系数与 Big-M 都有限且 Big-M 为正。核心关系与两个松弛右端直接来自符号文件里的
/// [`binaryzation_core_relations`]（`Indicator` / `SOS1` 已按 `mechanism_equivalent()` 归一到 `BigM`）。
///
/// 明确拒绝（回退 EAGER）：输入没有任何变量项（指示约束的线性表达式会变空，SDK 无法表达）、
/// Big-M 非正非有限、阈值或常数非有限。
///
/// **本函数不做盒证明**：松弛行的冗余需要读 SDK 列界，由 writer 在解析出列之后完成
/// （见 [`GurobiBinaryzationWriter::write_batch`]）。
///
/// Decide whether a binaryzation structure may be written natively and produce its write plan
/// (SDK-free).
///
/// Admitted shape: the input polynomial may be **any affine form** (`add_genconstr_indicator`'s condition
/// is a general linear expression, unlike ABS/extremum which only accept a column, so **no** bridge column
/// is needed and no public column changes), with at least one non-zero variable term and finite threshold,
/// input constant, coefficients and Big-M (which must be positive). The core relations and both relaxation
/// right-hand sides come from the symbol file's [`binaryzation_core_relations`] (`Indicator` / `SOS1` are
/// normalised to `BigM` by `mechanism_equivalent()`).
///
/// Explicitly rejected (kept on EAGER): an input without any variable term (the indicator constraint's
/// linear expression would be empty and the SDK cannot express it), a non-positive or non-finite Big-M, and
/// a non-finite threshold or constant.
///
/// This function performs **no box proof**: proving the relaxed rows redundant needs the SDK column bounds
/// and therefore happens in the writer once the columns are resolved (see
/// [`GurobiBinaryzationWriter::write_batch`]).
pub fn plan_binaryzation_native(
    structure: &BinaryzationStructure<f64>,
) -> std::result::Result<BinaryzationNativePlan, FallbackReason> {
    let name = structure.name();

    let big_m = structure.big_m();
    if !big_m.is_finite() || big_m <= 0.0 {
        return Err(FallbackReason::Rejected(format!(
            "binaryzation `{name}` native lowering requires a positive finite big-M, got {big_m}"
        )));
    }

    let threshold = *structure.threshold();
    if !threshold.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "binaryzation `{name}` native lowering requires a finite threshold, got {threshold}"
        )));
    }

    let input = structure.input_polynomial();
    let input_constant = *input.constant_term();
    if !input_constant.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "binaryzation `{name}` native lowering requires a finite input constant, got {input_constant}"
        )));
    }

    let mut coefficients = Vec::with_capacity(input.monomials().len());
    for monomial in input.monomials() {
        let coefficient = *monomial.coefficient();
        if !coefficient.is_finite() {
            return Err(FallbackReason::Rejected(format!(
                "binaryzation `{name}` native lowering requires finite input coefficients, got {coefficient}"
            )));
        }
        coefficients.push((monomial.var_index(), coefficient));
    }
    if !coefficients
        .iter()
        .any(|(_, coefficient)| *coefficient != 0.0)
    {
        return Err(FallbackReason::Rejected(format!(
            "binaryzation `{name}` native lowering requires at least one variable term in the input, got {} monomials with no non-zero coefficient",
            coefficients.len()
        )));
    }

    // 平移量与即时展开一致：`s = input - threshold`。
    // The shift matches the eager expansion: `s = input - threshold`.
    let constant = input_constant - threshold;
    if !constant.is_finite() {
        return Err(FallbackReason::Rejected(format!(
            "binaryzation `{name}` native lowering requires a finite shifted constant, got {constant}"
        )));
    }

    let core = binaryzation_core_relations(structure.method());
    Ok(BinaryzationNativePlan {
        name: name.to_string(),
        result: structure.result().clone(),
        coefficients,
        constant,
        big_m,
        when_true: core.when_true,
        when_false: core.when_false,
        relaxed_lower_rhs: core.relaxed_lower_rhs,
        relaxed_upper_rhs: core.relaxed_upper_rhs,
    })
}

/// Gurobi 的二值化原生 writer / Gurobi's native binaryzation writer.
///
/// 服务 [`BinaryzationStructure`]（`ospf-rust-core/src/symbol/functions/binaryzation.rs`）：即时展开的
/// 两行只有两条**不含 Big-M**的投影（取真一侧 `s >= ε_low`、取假一侧 `s <= -ε_up`），本 writer 用两条
/// `add_genconstr_indicator` 写它们，并在写入前证明两条 Big-M 松弛行在输入盒上冗余。
///
/// 门控与既有 writer 一致：结果列被固定、辅助列被外部引用（本结构的 `usage_binding` 上报 helpers
/// 为空，因此该门控恒真，保留只为统一拒绝语义）、结果列或条件列在求解模型中缺失、条件里出现指示列
/// 本身（即结果列）、指示列不是二元，都回退。本 writer 的独有门控是 **Big-M 冗余证明**（从 SDK 读条件
/// 列界求输入盒并按阈值平移；界不完整或松弛行不成立即回退，且显式比较 `grb::INFINITY`）。绝不写语义
/// 不完整的关系；真正的 SDK 写入失败返回 `Err`，由调用方对整模型回退。
///
/// Serves [`BinaryzationStructure`] (`ospf-rust-core/src/symbol/functions/binaryzation.rs`): only two of
/// the eager rows' projections carry no Big-M (true side `s >= ε_low`, false side `s <= -ε_up`), and this
/// writer writes them through two `add_genconstr_indicator` calls after proving the two Big-M relaxed rows
/// redundant on the input box.
///
/// The gates match the existing writers: a fixed result column, externally referenced helper columns (this
/// structure's `usage_binding` reports no helpers, so the gate is tautological and kept only for uniform
/// rejection semantics), a result or condition column missing from the solve model, the indicator column
/// (that is, the result column) appearing inside the condition, and a non-binary indicator column all force
/// a fallback. This writer's distinctive gate is the **Big-M redundancy proof** (it reads the condition
/// column bounds from the SDK, forms the input box and shifts it by the threshold; incomplete bounds or a
/// relaxed row that does not hold forces a fallback, with an explicit `grb::INFINITY` comparison). An
/// incomplete relation is never written, and a genuine SDK write failure returns `Err` so the caller can
/// fall back for the whole model.
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiBinaryzationWriter;

impl GurobiBinaryzationWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiBinaryzationWriter {
    fn name(&self) -> &str {
        "gurobi_binaryzation"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure
            .as_any()
            .downcast_ref::<BinaryzationStructure<f64>>()
            .is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        // 与 PWL / 指示 writer 同理：冗余证明与类型检查要读列属性，先落地一次待定变更。
        // As in the PWL and indicator writers: the redundancy proof and the type check read column
        // attributes, so the pending changes are flushed once first.
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_binaryzation writer failed to flush pending model changes before the big-M proof: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let structure = request
                .structure
                .as_any()
                .downcast_ref::<BinaryzationStructure<f64>>()
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "gurobi_binaryzation writer received a structure that is not a binaryzation structure"
                            .to_string(),
                    )
                })?;

            let plan = match plan_binaryzation_native(structure) {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native
            // relation meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 二值化不上报辅助列（`usage_binding` 的 helpers 为空），因此本门控恒真；保留它与其它
            // writer 保持同一套拒绝语义。
            // Binaryzation reports no helper columns (`usage_binding` has empty helpers), so this gate is
            // tautological; it is kept to preserve the same rejection semantics as the other writers.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let Some(indicator_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` indicator column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };

            let mut terms: Vec<(Var, f64)> = Vec::with_capacity(plan.coefficients.len());
            let mut missing_condition_column = false;
            for (index, coefficient) in &plan.coefficients {
                match container.variable_at(*index) {
                    Some(var) => terms.push((var, *coefficient)),
                    None => {
                        missing_condition_column = true;
                        break;
                    }
                }
            }
            if missing_condition_column {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` condition column is missing from the solve model",
                    plan.name
                ))));
                continue;
            }

            // 指示列不能同时出现在条件里：SDK 的指示约束以它作开关，把它再当作操作数写进去会让关系
            // 变成另一种含义（即时展开同样只把它当开关）。
            // The indicator column must not appear inside the condition: the SDK uses it as a switch, and
            // writing it as an operand as well would change the relation (eager expansion also uses it as a
            // switch only).
            if terms.iter().any(|(var, _)| *var == indicator_var) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` native lowering requires the indicator column to stay out of its own condition",
                    plan.name
                ))));
                continue;
            }

            // SDK 要求指示变量是二元变量。
            // The SDK requires the indicator variable to be binary.
            let indicator_type = container
                .model_mut()
                .get_obj_attr(attr::VType, &indicator_var);
            match indicator_type {
                Ok(VarType::Binary) => {}
                Ok(other) => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "binaryzation `{}` native lowering requires a binary indicator column, got {other:?}",
                        plan.name
                    ))));
                    continue;
                }
                Err(error) => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "binaryzation `{}` native lowering could not read the indicator column type from the Gurobi model: {error}",
                        plan.name
                    ))));
                    continue;
                }
            }

            // Big-M 冗余证明：从列界算出 `s = Σ c_k x_k + constant` 在盒上的上下界。界不完整
            // （列无界 / 读不到）时无法证明等价性，保守回退而不是冒险写入。
            // Big-M redundancy proof: derive the box bounds of `s = Σ c_k x_k + constant` from the column
            // bounds. Incomplete bounds (unbounded or unreadable columns) mean equivalence cannot be
            // proven, so the writer conservatively falls back instead of writing on faith.
            let mut s_min = plan.constant;
            let mut s_max = plan.constant;
            let mut incomplete_bounds = None;
            for (var, coefficient) in &terms {
                let bounds = {
                    let model = container.model_mut();
                    match model.get_obj_attr(attr::LB, var) {
                        Ok(lb) => model
                            .get_obj_attr(attr::UB, var)
                            .map(|ub| (lb, ub))
                            .map_err(|error| error.to_string()),
                        Err(error) => Err(error.to_string()),
                    }
                };
                let (lower_bound, upper_bound) = match bounds {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        incomplete_bounds = Some(format!(
                            "could not read the condition column bounds from the Gurobi model: {error}"
                        ));
                        break;
                    }
                };
                // Gurobi 用 ±1e100（`grb::INFINITY`）表示无穷界，它本身是有限数，因此必须显式比较。
                // Gurobi represents infinite bounds as ±1e100 (`grb::INFINITY`), which are finite numbers,
                // so they must be compared explicitly.
                if !lower_bound.is_finite()
                    || !upper_bound.is_finite()
                    || lower_bound <= -INFINITY
                    || upper_bound >= INFINITY
                {
                    incomplete_bounds = Some(format!(
                        "condition column bounds [{lower_bound}, {upper_bound}] are not a finite interval"
                    ));
                    break;
                }
                if *coefficient >= 0.0 {
                    s_min += coefficient * lower_bound;
                    s_max += coefficient * upper_bound;
                } else {
                    s_min += coefficient * upper_bound;
                    s_max += coefficient * lower_bound;
                }
            }
            if let Some(reason) = incomplete_bounds {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }
            if let Err(reason) = plan.prove_big_m_relaxation(s_min, s_max) {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "binaryzation `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 两条核心投影分别写成一条指示约束：`ind = 1` 与 `ind = 0`。SDK 写入失败直接返回错误：
            // 调用方必须对整模型回退，而不是保留半写入状态。
            // The two core projections become one indicator constraint each: `ind = 1` and `ind = 0`. An SDK
            // write failure returns an error: the caller must fall back for the whole model instead of
            // keeping a half-written state.
            for (indicator_value, (relation, rhs), suffix) in [
                (true, plan.when_true, "true"),
                (false, plan.when_false, "false"),
            ] {
                let condition = indicator_condition(&terms, plan.constant, relation, rhs);
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_binaryzation_{suffix}", plan.name),
                        indicator_var,
                        indicator_value,
                        condition,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_binaryzation writer failed to write binaryzation `{}` ({suffix}): {error}",
                            plan.name
                        ))
                    })?;
            }

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_BINARYZATION_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

/// 蕴含原生写入的 schema 版本 / Schema version of the native implication write.
///
/// 与关系指示分开一个 schema：蕴含的原生结构不只是「一条关系 = 两条指示」，而是「两个内部关系指示器
/// 各自的指示 + 3 条耦合指示」，并且辅助列非空；schema 变化表示结构含义变化，恢复阶段必须拒绝旧指纹。
///
/// A separate schema from the relation indicator: the native implication structure is not just "one
/// relation = two indicators" but "each of the two internal relation indicators plus three coupling
/// indicators", and it carries non-empty helper columns; a schema change means the structure's meaning
/// changed and recovery must reject older fingerprints.
pub const GUROBI_IMPLY_SCHEMA: &str = "functions-imply-1";

/// 蕴含原生写入计划 / Plan for one native implication write.
///
/// 即时展开 = 前提指示器的行 + 结论指示器的行 + 4 条耦合逻辑行（`r ≥ c`、`r + p ≥ 1`、
/// `r + p − c ≤ 1`、`r ≤ 1`）。原生写入逐项覆盖：
///
/// - 两个子指示器各自复用 [`plan_indicator_from_parts`]，因此它们的核心行与严格性 ε 和第 1 批**同源**，
///   各自的 M 取结构里冻结的字段（不重新推断），松弛行由各自的 [`IndicatorNativePlan::prove_big_m_relaxation`]
///   证明；
/// - 4 条耦合行由 [`imply_coupling_indicators`] 的 3 条指示约束重建。**这不是逐行对应**：即时的
///   `r ≥ c` 是无条件行，而耦合表只在 `c = 1` 时给 `r ≥ 1`，`c = 0` 侧的 `r ≥ 0` 由结果列的二元类型
///   自动成立；`r ≤ 1` 同样由二元类型给出，因此可以省略。等价性因此是**二元列上的点集等价**，
///   writer 必须显式校验 `r`、`p`、`c` 都是二元列（见 [`GurobiImplyWriter::write_batch`]）。
///
/// The eager expansion is the premise indicator's rows plus the consequence indicator's rows plus four
/// coupling rows (`r ≥ c`, `r + p ≥ 1`, `r + p − c ≤ 1`, `r ≤ 1`). The native write covers each part:
///
/// - both sub-indicators reuse [`plan_indicator_from_parts`], so their core rows and strictness ε are the
///   **same** as batch 1's, each takes the Big-M frozen in the structure (never re-inferred), and each
///   proves its relaxed rows through [`IndicatorNativePlan::prove_big_m_relaxation`];
/// - the four coupling rows are rebuilt by the three indicator constraints of
///   [`imply_coupling_indicators`]. **This is not a row-by-row correspondence**: the eager `r ≥ c` is
///   unconditional while the table only gives `r ≥ 1` at `c = 1`, and the `c = 0` side of `r ≥ c` (that is
///   `r ≥ 0`) follows from the binary type of the result column; `r ≤ 1` likewise comes from that binary
///   type and can be omitted. The equivalence is therefore a **point-set equivalence on binary columns**, and
///   the writer must verify that `r`, `p` and `c` are all binary (see
///   [`GurobiImplyWriter::write_batch`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ImplyNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列 `r` / Result column `r`
    pub result: VariableId,
    /// 前提子指示器的计划；其 `result` 同时是前提指示列 `p`
    /// Plan of the premise sub-indicator; its `result` is also the premise indicator column `p`
    pub premise: IndicatorNativePlan,
    /// 结论子指示器的计划；其 `result` 同时是结论指示列 `c`
    /// Plan of the consequence sub-indicator; its `result` is also the consequence indicator column `c`
    pub consequence: IndicatorNativePlan,
}

/// 判断一个蕴含结构是否可以原生写入，并给出写入计划（不依赖 SDK）。
///
/// 准入：两个子指示器的关系类型都必须是 `<=` / `>=` / `<` / `>`（`=` / `!=` 的取假侧是析取，需要
/// side 辅助列做情形分裂，两条指示约束表达不了，因此整个结构回退），子指示器的左式/右式/系数有限、
/// 至少有一个变量项，两个冻结 Big-M 都是正有限值。
///
/// **本函数不做盒证明与列类型校验**：两者的松弛行冗余需要读 SDK 列界，二元类型需要读列属性，都由
/// writer 在解析出列之后完成（见 [`GurobiImplyWriter::write_batch`]）。
///
/// Decide whether an implication structure may be written natively and produce its write plan (SDK-free).
///
/// Admission: both sub-indicators' relations must be `<=` / `>=` / `<` / `>` (`=` / `!=` have a disjunctive
/// false side that needs the side helper column for a case split, which two indicator constraints cannot
/// express, so the whole structure falls back), their left/right values and coefficients must be finite with
/// at least one variable term, and both frozen Big-M values must be positive and finite.
///
/// This function performs **no box proof and no column type check**: proving the relaxed rows redundant needs
/// the SDK column bounds and the binary types need column attributes, both of which the writer does once the
/// columns are resolved (see [`GurobiImplyWriter::write_batch`]).
pub fn plan_imply_native(
    structure: &ImplyStructure<f64>,
) -> std::result::Result<ImplyNativePlan, FallbackReason> {
    let name = structure.name();

    // 两个子指示器各自复用第 1 批的准入与映射；前缀不同以保证原生约束名不冲突。
    // Both sub-indicators reuse batch 1's admission and mapping; distinct prefixes keep the native
    // constraint names apart.
    let premise = plan_indicator_from_parts(
        &format!("{name}_premise"),
        structure.premise_indicator().inequality_kind(),
        structure.premise_big_m(),
        structure.premise_indicator().left_polynomial(),
        *structure.premise_indicator().right_value(),
        structure.premise_indicator().result_variable().id(),
    )
    .map_err(|reason| {
        FallbackReason::Rejected(format!(
            "imply `{name}` native lowering cannot write its premise sub-indicator: {reason:?}"
        ))
    })?;

    let consequence = plan_indicator_from_parts(
        &format!("{name}_consequence"),
        structure.consequence_indicator().inequality_kind(),
        structure.consequence_big_m(),
        structure.consequence_indicator().left_polynomial(),
        *structure.consequence_indicator().right_value(),
        structure.consequence_indicator().result_variable().id(),
    )
    .map_err(|reason| {
        FallbackReason::Rejected(format!(
            "imply `{name}` native lowering cannot write its consequence sub-indicator: {reason:?}"
        ))
    })?;

    let result = structure.result().clone();
    if premise.result == result || consequence.result == result {
        return Err(FallbackReason::Rejected(format!(
            "imply `{name}` native lowering requires the result column to stay distinct from both sub-indicator columns"
        )));
    }
    if premise.result == consequence.result {
        return Err(FallbackReason::Rejected(format!(
            "imply `{name}` native lowering requires the premise and consequence columns to stay distinct"
        )));
    }

    Ok(ImplyNativePlan {
        name: name.to_string(),
        result,
        premise,
        consequence,
    })
}

/// Gurobi 的蕴含原生 writer / Gurobi's native implication writer.
///
/// 服务 [`ImplyStructure`]（`ospf-rust-core/src/symbol/functions/imply.rs`）：先按两个子指示器的计划各写
/// 两条指示约束（`ind = 1` / `ind = 0`），再用 [`imply_coupling_indicators`] 的 3 条耦合指示约束把结果列
/// 钉成 `r = max(c, 1 − p)`。耦合行没有 Big-M、没有额外容差；两个子指示器的松弛行分别用各自冻结的 M 与
/// 输入盒做冗余证明（与第 1 批同一个证明函数与同一个容差常量）。
///
/// 门控：结果列被固定、辅助列（两个子指示器的结果列与 side 列）被外部引用、结果列 / 子指示器列 /
/// 条件列在求解模型中缺失、条件里出现该子指示器自己的指示列、`r` / `p` / `c` 任一不是二元列，都回退。
/// **二元校验是必须的而不是防御性的**：耦合等价依赖二元性（`c = 0` 侧的 `r ≥ c` 与 `r ≤ 1` 由二元类型
/// 给出），非二元列上原生写入与即时展开会真的分歧（`imply.rs` 的机械化单测给出反例）。
///
/// Serves [`ImplyStructure`] (`ospf-rust-core/src/symbol/functions/imply.rs`): it first writes two indicator
/// constraints per sub-indicator (`ind = 1` / `ind = 0`) from the sub-plans, then pins the result to
/// `r = max(c, 1 − p)` with the three coupling indicators of [`imply_coupling_indicators`]. The coupling
/// carries no Big-M and no extra tolerance, while each sub-indicator's relaxed rows are proven redundant
/// with its own frozen Big-M and input box (the same proof function and tolerance constant as batch 1).
///
/// Gates: a fixed result column, externally referenced helper columns (both sub-indicators' result and side
/// columns), a result / sub-indicator / condition column missing from the solve model, a condition containing
/// the sub-indicator's own indicator column, and any of `r` / `p` / `c` not being binary all force a fallback.
/// The **binary check is mandatory rather than defensive**: the coupling equivalence relies on binariness (the
/// `c = 0` side of `r ≥ c` and `r ≤ 1` come from the binary type), and on non-binary columns the native write
/// and eager expansion really do diverge (the mechanical unit test in `imply.rs` gives the counterexample).
#[derive(Debug, Default, Clone, Copy)]
pub struct GurobiImplyWriter;

impl GurobiImplyWriter {
    /// 创建 writer / Create the writer.
    pub fn new() -> Self {
        Self
    }
}

impl NativeFunctionWriter<GurobiNativeContainer, f64> for GurobiImplyWriter {
    fn name(&self) -> &str {
        "gurobi_imply"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure
            .as_any()
            .downcast_ref::<ImplyStructure<f64>>()
            .is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer,
        requests: &[NativeWriteRequest<'_, f64>],
    ) -> Result<Option<Vec<NativeWriteOutcome>>> {
        if requests.is_empty() {
            return Ok(None);
        }

        // 与其它 writer 同理：列类型与列界读取需要先把待定变更落地。
        // As in the other writers: reading column types and bounds needs the pending changes flushed first.
        container.model_mut().update().map_err(|error| {
            ModelError::InvalidConstraint(format!(
                "gurobi_imply writer failed to flush pending model changes before the checks: {error}"
            ))
        })?;

        let mut outcomes = Vec::with_capacity(requests.len());
        for request in requests {
            let structure = request
                .structure
                .as_any()
                .downcast_ref::<ImplyStructure<f64>>()
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "gurobi_imply writer received a structure that is not an implication structure"
                            .to_string(),
                    )
                })?;

            let plan = match plan_imply_native(structure) {
                Ok(plan) => plan,
                Err(reason) => {
                    outcomes.push(NativeWriteOutcome::Fallback(reason));
                    continue;
                }
            };

            // 结果被固定时禁止原生写入：固定列的代换会让原生关系失去意义。
            // A fixed result forbids a native write: substituting the column would make the native relation
            // meaningless.
            if request.usage.forbids_native_write() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` native lowering rejected a fixed result column",
                    plan.name
                ))));
                continue;
            }

            // 两个子指示器的结果列（以及等号形态下的 side 列）是本结构的辅助列：一旦被外部引用，原生写入
            // 与即时展开在这些列上的含义就可能被外部行区分出来。
            // Both sub-indicators' result columns (plus their side columns in the equality form) are helpers
            // of this structure: once referenced elsewhere, an external row could tell the native write and
            // eager expansion apart on them.
            if !request.usage.helpers_are_exclusive() {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` native lowering rejected externally referenced helper columns",
                    plan.name
                ))));
                continue;
            }

            let Some(result_var) = container.variable(&plan.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` result column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };
            let Some(premise_var) = container.variable(&plan.premise.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` premise indicator column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };
            let Some(consequence_var) = container.variable(&plan.consequence.result) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` consequence indicator column is missing from the solve model",
                    plan.name
                ))));
                continue;
            };

            // 两个子指示器的条件列：分别解析，缺失或含自己的指示列即回退。
            // The two sub-indicators' condition columns: resolved separately; a missing column or one
            // containing the sub-indicator's own indicator column forces a fallback.
            let mut sub_terms: Vec<(Var, Vec<(Var, f64)>)> = Vec::with_capacity(2);
            let mut condition_problem = None;
            for (sub_plan, indicator_var, label) in [
                (&plan.premise, premise_var, "premise"),
                (&plan.consequence, consequence_var, "consequence"),
            ] {
                let mut terms = Vec::with_capacity(sub_plan.coefficients.len());
                for (index, coefficient) in &sub_plan.coefficients {
                    match container.variable_at(*index) {
                        Some(var) => terms.push((var, *coefficient)),
                        None => {
                            condition_problem =
                                Some(format!("{label} condition column is missing from the solve model"));
                            break;
                        }
                    }
                }
                if condition_problem.is_some() {
                    break;
                }
                if terms.iter().any(|(var, _)| *var == indicator_var) {
                    condition_problem = Some(format!(
                        "{label} condition contains its own indicator column"
                    ));
                    break;
                }
                sub_terms.push((indicator_var, terms));
            }
            let (premise_terms, consequence_terms) = match sub_terms.as_slice() {
                [(_, premise_terms), (_, consequence_terms)] => {
                    (premise_terms.clone(), consequence_terms.clone())
                }
                _ => {
                    outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                        "imply `{}` native lowering cannot resolve its sub-indicator conditions: {}",
                        plan.name,
                        condition_problem.unwrap_or_else(|| "unknown reason".to_string())
                    ))));
                    continue;
                }
            };

            // `r` / `p` / `c` 必须都是二元列：耦合等价依赖二元性（见 writer 文档）。
            // `r` / `p` / `c` must all be binary: the coupling equivalence relies on binariness (see the
            // writer documentation).
            let mut non_binary = None;
            for (var, label) in [
                (result_var, "result"),
                (premise_var, "premise indicator"),
                (consequence_var, "consequence indicator"),
            ] {
                match container.model_mut().get_obj_attr(attr::VType, &var) {
                    Ok(VarType::Binary) => {}
                    Ok(other) => {
                        non_binary = Some(format!("{label} column is {other:?}"));
                        break;
                    }
                    Err(error) => {
                        non_binary = Some(format!(
                            "could not read the {label} column type from the Gurobi model: {error}"
                        ));
                        break;
                    }
                }
            }
            if let Some(reason) = non_binary {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` native lowering requires binary r/p/c columns: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 两个子指示器各自的 Big-M 冗余证明：读条件列界求各自的输入盒再平移。
            // Each sub-indicator's Big-M redundancy proof: read the condition column bounds, form that
            // sub-indicator's own input box and shift it.
            let mut proof_problem = None;
            for (sub_plan, terms) in [
                (&plan.premise, &premise_terms),
                (&plan.consequence, &consequence_terms),
            ] {
                let mut s_min = sub_plan.constant;
                let mut s_max = sub_plan.constant;
                for (var, coefficient) in terms {
                    let bounds = {
                        let model = container.model_mut();
                        match model.get_obj_attr(attr::LB, var) {
                            Ok(lb) => model
                                .get_obj_attr(attr::UB, var)
                                .map(|ub| (lb, ub))
                                .map_err(|error| error.to_string()),
                            Err(error) => Err(error.to_string()),
                        }
                    };
                    let (lower_bound, upper_bound) = match bounds {
                        Ok(bounds) => bounds,
                        Err(error) => {
                            proof_problem = Some(format!(
                                "could not read the condition column bounds from the Gurobi model: {error}"
                            ));
                            break;
                        }
                    };
                    // Gurobi 用 ±1e100（`grb::INFINITY`）表示无穷界，它本身是有限数，因此必须显式比较。
                    // Gurobi represents infinite bounds as ±1e100 (`grb::INFINITY`), which are finite, so
                    // they must be compared explicitly.
                    if !lower_bound.is_finite()
                        || !upper_bound.is_finite()
                        || lower_bound <= -INFINITY
                        || upper_bound >= INFINITY
                    {
                        proof_problem = Some(format!(
                            "condition column bounds [{lower_bound}, {upper_bound}] are not a finite interval"
                        ));
                        break;
                    }
                    if *coefficient >= 0.0 {
                        s_min += coefficient * lower_bound;
                        s_max += coefficient * upper_bound;
                    } else {
                        s_min += coefficient * upper_bound;
                        s_max += coefficient * lower_bound;
                    }
                }
                if proof_problem.is_some() {
                    break;
                }
                if let Err(reason) = sub_plan.prove_big_m_relaxation(s_min, s_max) {
                    proof_problem = Some(reason);
                    break;
                }
            }
            if let Some(reason) = proof_problem {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "imply `{}` native lowering cannot prove the big-M relaxation: {reason}",
                    plan.name
                ))));
                continue;
            }

            // 两个子指示器各写两条指示约束。
            // Two indicator constraints per sub-indicator.
            let mut write_failure = None;
            for (sub_plan, indicator_var, terms) in [
                (&plan.premise, premise_var, &premise_terms),
                (&plan.consequence, consequence_var, &consequence_terms),
            ] {
                for (indicator_value, (relation, rhs), suffix) in [
                    (true, sub_plan.when_true, "true"),
                    (false, sub_plan.when_false, "false"),
                ] {
                    let condition = indicator_condition(terms, sub_plan.constant, relation, rhs);
                    if let Err(error) = container.model_mut().add_genconstr_indicator(
                        &format!("{}_indicator_{suffix}", sub_plan.name),
                        indicator_var,
                        indicator_value,
                        condition,
                    ) {
                        write_failure = Some(format!(
                            "failed to write `{}` ({suffix}): {error}",
                            sub_plan.name
                        ));
                        break;
                    }
                }
                if write_failure.is_some() {
                    break;
                }
            }
            if let Some(reason) = write_failure {
                return Err(ModelError::InvalidConstraint(format!(
                    "gurobi_imply writer {reason}"
                ))
                .into());
            }

            // 3 条耦合指示约束：`r + other_coefficient · other REL rhs`，指示列取自前提或结论列。
            // Three coupling indicator constraints: `r + other_coefficient · other REL rhs`, keyed on the
            // premise or the consequence column.
            for (index, indicator) in imply_coupling_indicators().iter().enumerate() {
                let (key_var, other_var) = if indicator.keyed_on_premise {
                    (premise_var, consequence_var)
                } else {
                    (consequence_var, premise_var)
                };
                let mut terms = vec![(result_var, 1.0)];
                if indicator.other_coefficient != 0.0 {
                    terms.push((other_var, indicator.other_coefficient));
                }
                let condition =
                    indicator_condition(&terms, 0.0, indicator.relation, indicator.rhs);
                container
                    .model_mut()
                    .add_genconstr_indicator(
                        &format!("{}_coupling_{index}", plan.name),
                        key_var,
                        indicator.indicator_value,
                        condition,
                    )
                    .map_err(|error| {
                        ModelError::InvalidConstraint(format!(
                            "gurobi_imply writer failed to write implication `{}` coupling {index}: {error}",
                            plan.name
                        ))
                    })?;
            }

            outcomes.push(NativeWriteOutcome::Native(NativeWriteRecord::new(
                self.name(),
                GUROBI_IMPLY_SCHEMA,
            )));
        }

        Ok(Some(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinearConstraint, LinearInequality};
    use crate::model::flatten::{Linear, LinearMonomial};
    use crate::symbol::function::{
        AbsFunction, CosFunction, Point2, SigmoidFunction, SinFunction,
    };
    use std::sync::Arc;

    fn abs_structure(
        input: Linear<f64>,
        result: VariableId,
        side: VariableId,
    ) -> AbsStructure<f64> {
        AbsStructure::new(
            "abs_native_plan",
            input.clone(),
            result,
            side,
            crate::symbol::function::AbsBranchBigM::fallback(),
            Arc::new(AbsFunction::<f64>::new(120, "abs_native_plan", input)),
        )
    }

    fn extremum_structure(
        id: u64,
        name: &str,
        polynomials: Vec<Linear<f64>>,
        minimum: bool,
        result: VariableId,
    ) -> std::sync::Arc<dyn DeferredFunctionStructure<f64>> {
        let big_ms = vec![10.0; polynomials.len()];
        if minimum {
            std::sync::Arc::new(MinStructure::new(
                name,
                Arc::new(crate::symbol::function::MinFunction::<f64>::new(
                    id,
                    name,
                    polynomials,
                    true,
                )),
                big_ms,
            ))
        } else {
            std::sync::Arc::new(MaxStructure::new(
                name,
                Arc::new(crate::symbol::function::MaxFunction::<f64>::new(
                    id,
                    name,
                    polynomials,
                    true,
                )),
                big_ms,
            ))
        }
    }

    #[test]
    fn extremum_plan_admits_columns_and_a_shared_constant() {
        let structure = extremum_structure(
            121,
            "max_native_plan",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 3)], 2.0),
                Linear::new(vec![LinearMonomial::new(1.0, 7)], 2.0),
            ],
            false,
            VariableId::standalone(8_100),
        );
        let max_structure = structure
            .as_any()
            .downcast_ref::<MaxStructure<f64>>()
            .expect("structure should be a MAX structure");

        let plan = plan_max_native(max_structure).expect("column candidates with a shared constant");
        assert_eq!(plan.name, "max_native_plan");
        assert_eq!(plan.operand_indices, vec![3, 7]);
        assert_eq!(plan.constant, Some(2.0));
        assert!(!plan.minimum);

        let min_structure = extremum_structure(
            122,
            "min_native_plan",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 4)], 0.0)],
            true,
            VariableId::standalone(8_101),
        );
        let min_structure = min_structure
            .as_any()
            .downcast_ref::<MinStructure<f64>>()
            .expect("structure should be a MIN structure");
        let plan = plan_min_native(min_structure).expect("a bare column candidate");
        assert_eq!(plan.operand_indices, vec![4]);
        assert_eq!(plan.constant, None);
        assert!(plan.minimum);
    }

    #[test]
    fn extremum_plan_rejects_shapes_the_native_relation_cannot_express() {
        // 一般仿射候选需要额外的桥接列，原生接口只接受列作为操作数。
        // A general affine candidate would need an extra bridge column; the native interface only
        // accepts columns as operands.
        let affine = extremum_structure(
            123,
            "max_affine",
            vec![Linear::new(
                vec![LinearMonomial::new(2.0, 0), LinearMonomial::new(1.0, 1)],
                0.0,
            )],
            false,
            VariableId::standalone(8_110),
        );
        let affine = affine
            .as_any()
            .downcast_ref::<MaxStructure<f64>>()
            .unwrap();
        assert!(plan_max_native(affine).is_err());

        // 系数不为 1 的单项式同样无法直接写成原生操作数。
        // A monomial with a coefficient other than one cannot become a native operand either.
        let scaled = extremum_structure(
            124,
            "max_scaled",
            vec![Linear::new(vec![LinearMonomial::new(3.0, 0)], 0.0)],
            false,
            VariableId::standalone(8_111),
        );
        let scaled = scaled
            .as_any()
            .downcast_ref::<MaxStructure<f64>>()
            .unwrap();
        assert!(plan_max_native(scaled).is_err());

        // 各候选常数不同时，单一共享常数无法表达。
        // Distinct candidate constants cannot be expressed by the single shared constant.
        let mixed = extremum_structure(
            125,
            "max_mixed_constants",
            vec![
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
                Linear::new(vec![LinearMonomial::new(1.0, 1)], 5.0),
            ],
            false,
            VariableId::standalone(8_112),
        );
        let mixed = mixed
            .as_any()
            .downcast_ref::<MaxStructure<f64>>()
            .unwrap();
        assert!(plan_max_native(mixed).is_err());
    }

    #[test]
    fn extremum_writers_claim_only_their_own_structure_kind() {
        let max_structure = extremum_structure(
            126,
            "max_claim",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            false,
            VariableId::standalone(8_120),
        );
        let min_structure = extremum_structure(
            127,
            "min_claim",
            vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)],
            true,
            VariableId::standalone(8_121),
        );

        let max_writer = GurobiExtremumWriter::maximum();
        let min_writer = GurobiExtremumWriter::minimum();
        assert_eq!(max_writer.name(), "gurobi_max");
        assert_eq!(min_writer.name(), "gurobi_min");
        assert!(max_writer.supports(max_structure.as_ref()));
        assert!(!max_writer.supports(min_structure.as_ref()));
        assert!(min_writer.supports(min_structure.as_ref()));
        assert!(!min_writer.supports(max_structure.as_ref()));
    }

    #[test]
    fn native_plan_accepts_a_direct_variable_input() {
        let result = VariableId::standalone(8_000);
        let argument = VariableId::standalone(8_001);
        let structure = abs_structure(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            result.clone(),
            argument.clone(),
        );

        let plan = plan_abs_native(&structure).expect("direct variable input should be admitted");
        assert_eq!(plan.name, "abs_native_plan");
        assert_eq!(plan.result, result);
        // 参数是输入单项式指向的列下标（此处输入是第 0 列），不是分支指示列。
        // The argument is the column index the input monomial points at (column 0 here), not the
        // branch selector column.
        assert_eq!(plan.argument_index, 0);
    }

    #[test]
    fn native_plan_rejects_inputs_that_need_a_bridge_column() {
        let result = VariableId::standalone(8_010);
        let side = VariableId::standalone(8_011);

        // 多个单项式：需要桥接列 `p = a·x + b·z + c`。
        // More than one monomial: a bridge column `p = a·x + b·z + c` would be required.
        let multi = abs_structure(
            Linear::new(
                vec![
                    LinearMonomial::new(1.0, 0),
                    LinearMonomial::new(1.0, 1),
                ],
                0.0,
            ),
            result.clone(),
            side.clone(),
        );
        assert!(plan_abs_native(&multi).is_err());

        // 非 1 系数：缩放会改变结果含义。
        // Non-unit coefficient: the scaling would change the result's meaning.
        let scaled = abs_structure(
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 0.0),
            result.clone(),
            side.clone(),
        );
        assert!(plan_abs_native(&scaled).is_err());

        // 非零常数项：同样需要桥接列。
        // A non-zero constant also needs a bridge column.
        let shifted = abs_structure(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0),
            result.clone(),
            side.clone(),
        );
        assert!(plan_abs_native(&shifted).is_err());

        // 空输入多项式同样拒绝。
        // An empty input polynomial is rejected as well.
        let empty = abs_structure(Linear::new(Vec::new(), 0.0), result.clone(), side.clone());
        assert!(plan_abs_native(&empty).is_err());
    }

    #[test]
    fn native_plan_rejects_a_self_referencing_column() {
        let shared = VariableId::standalone(8_020);
        let structure = abs_structure(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            shared.clone(),
            shared,
        );
        assert!(plan_abs_native(&structure).is_err());
    }

    #[test]
    fn gurobi_abs_writer_claims_only_abs_structures() {
        let writer = GurobiAbsWriter::new();
        assert_eq!(writer.name(), "gurobi_abs");

        let structure = abs_structure(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            VariableId::standalone(8_030),
            VariableId::standalone(8_031),
        );
        assert!(writer.supports(&structure));
    }

    fn sin_native_structure(
        id: u64,
        name: &str,
        input: Linear<f64>,
    ) -> SinStructure<f64> {
        SinStructure::new(name, Arc::new(SinFunction::<f64>::new(id, name, input)))
    }

    fn cos_native_structure(
        id: u64,
        name: &str,
        input: Linear<f64>,
    ) -> CosStructure<f64> {
        CosStructure::new(name, Arc::new(CosFunction::<f64>::new(id, name, input)))
    }

    fn sigmoid_native_structure(
        id: u64,
        name: &str,
        input: Linear<f64>,
        points: Vec<Point2<f64>>,
    ) -> SigmoidStructure<f64> {
        SigmoidStructure::new(
            name,
            Arc::new(SigmoidFunction::<f64>::with_points(id, name, input, points)),
        )
    }

    #[test]
    fn pwl_plan_admits_a_column_input_and_a_monotone_point_table() {
        let input = Linear::new(vec![LinearMonomial::new(1.0, 3)], 0.0);
        let sin = sin_native_structure(130, "sin_native_plan", input.clone());
        let plan = plan_sin_native(&sin).expect("a direct variable input should be admitted");

        assert_eq!(plan.shape, "sin");
        assert_eq!(plan.name, "sin_native_plan");
        assert_eq!(plan.result, *sin.result());
        // 输入是 `1 · x_3 + 0`，因此输入列下标就是 3。
        // The input is `1 · x_3 + 0`, so the input column index is exactly 3.
        assert_eq!(plan.argument_index, 3);
        // 点表必须与结构上报的断点/函数值完全一致：原生写入与即时展开描述同一条分段线性函数。
        // The table must equal the structure's breakpoints and function values: the native write and
        // eager expansion describe the same piecewise-linear function.
        assert_eq!(plan.points, sin.points());
        assert_eq!(plan.points.len(), 33);
        assert_eq!(plan.points[0].0, -std::f64::consts::PI);
        assert_eq!(plan.points[plan.points.len() - 1].0, std::f64::consts::PI);
        assert!(
            plan.points.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "the admitted point table must have strictly increasing x values"
        );

        let cos = cos_native_structure(131, "cos_native_plan", input.clone());
        let cos_plan = plan_cos_native(&cos).expect("cos should share the same admission");
        assert_eq!(cos_plan.shape, "cos");
        assert_eq!(cos_plan.result, *cos.result());
        assert_eq!(cos_plan.argument_index, 3);
        assert_eq!(cos_plan.points, cos.points());
        // 断点区间是 [-π, π]，因此 cos 的首点是 (-π, -1)。
        // The breakpoint interval is [-π, π], so cos's first point is (-π, -1).
        assert_eq!(cos_plan.points[0].1, -1.0);

        // Sigmoid 的点表来自它自己的采样点，取值方式与 Point2 的公开字段一致。
        // The sigmoid table comes from its own sampling points and reads Point2's public fields.
        let sigmoid = sigmoid_native_structure(
            132,
            "sigmoid_native_plan",
            input,
            vec![
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.5),
                Point2::new(3.0, 1.0),
            ],
        );
        let sigmoid_plan =
            plan_sigmoid_native(&sigmoid).expect("sigmoid should share the same admission");
        assert_eq!(sigmoid_plan.shape, "sigmoid");
        assert_eq!(sigmoid_plan.result, *sigmoid.result());
        assert_eq!(sigmoid_plan.argument_index, 3);
        assert_eq!(sigmoid_plan.points, vec![(0.0, 0.0), (1.0, 0.5), (3.0, 1.0)]);
    }

    #[test]
    fn pwl_plan_rejects_inputs_that_need_a_bridge_column() {
        let result = VariableId::standalone(8_200);
        let points = vec![(0.0, 0.0), (1.0, 1.0)];

        // 多个单项式：需要桥接列 `p = a·x + b·z + c`。
        // More than one monomial: a bridge column `p = a·x + b·z + c` would be required.
        let multi = Linear::new(
            vec![
                LinearMonomial::new(1.0, 0),
                LinearMonomial::new(1.0, 1),
            ],
            0.0,
        );
        assert!(plan_pwl_native("sin", "pwl_multi", &result, &multi, points.clone()).is_err());

        // 非 1 系数：缩放会改变分段线性函数的自变量刻度。
        // Non-unit coefficient: the scaling would change the argument scale of the PWL function.
        let scaled = Linear::new(vec![LinearMonomial::new(2.0, 0)], 0.0);
        assert!(plan_pwl_native("sin", "pwl_scaled", &result, &scaled, points.clone()).is_err());

        // 非零常数项：同样需要桥接列。
        // A non-zero constant also needs a bridge column.
        let shifted = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);
        assert!(plan_pwl_native("sin", "pwl_shifted", &result, &shifted, points.clone()).is_err());

        // 空输入多项式同样拒绝。
        // An empty input polynomial is rejected as well.
        let empty = Linear::new(Vec::new(), 0.0);
        assert!(plan_pwl_native("sin", "pwl_empty", &result, &empty, points).is_err());
    }

    #[test]
    fn pwl_plan_rejects_point_tables_it_cannot_write() {
        let result = VariableId::standalone(8_210);
        let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);

        // 点数不足：Gurobi 无法定义分段线性函数。
        // Too few points: Gurobi cannot define a piecewise-linear function.
        assert!(
            plan_pwl_native("sin", "pwl_one_point", &result, &input, vec![(0.0, 0.0)]).is_err()
        );
        assert!(
            plan_pwl_native("sin", "pwl_no_point", &result, &input, Vec::new()).is_err()
        );

        // x 非严格递增（含相等与逆序）：点表无法作为分段线性函数的定义。
        // x not strictly increasing (equal or reversed): the table cannot define the PWL function.
        assert!(
            plan_pwl_native(
                "cos",
                "pwl_equal_x",
                &result,
                &input,
                vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
            )
            .is_err()
        );
        assert!(
            plan_pwl_native(
                "cos",
                "pwl_reversed_x",
                &result,
                &input,
                vec![(1.0, 0.0), (0.0, 1.0)],
            )
            .is_err()
        );

        // 非有限坐标：无法写出确定的函数值。
        // Non-finite coordinates: no definite function value can be written.
        assert!(
            plan_pwl_native(
                "sigmoid",
                "pwl_nan",
                &result,
                &input,
                vec![(0.0, 0.0), (1.0, f64::NAN)],
            )
            .is_err()
        );
    }

    #[test]
    fn pwl_writer_claims_only_piecewise_linear_structures() {
        let writer = GurobiPwlWriter::new();
        assert_eq!(writer.name(), "gurobi_pwl");

        let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let sin: std::sync::Arc<dyn DeferredFunctionStructure<f64>> =
            std::sync::Arc::new(sin_native_structure(140, "sin_claim", input.clone()));
        let cos: std::sync::Arc<dyn DeferredFunctionStructure<f64>> =
            std::sync::Arc::new(cos_native_structure(141, "cos_claim", input.clone()));
        let sigmoid: std::sync::Arc<dyn DeferredFunctionStructure<f64>> =
            std::sync::Arc::new(sigmoid_native_structure(
                142,
                "sigmoid_claim",
                input.clone(),
                vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)],
            ));
        let abs: std::sync::Arc<dyn DeferredFunctionStructure<f64>> = std::sync::Arc::new(
            abs_structure(
                input,
                VariableId::standalone(8_220),
                VariableId::standalone(8_221),
            ),
        );

        assert!(writer.supports(sin.as_ref()));
        assert!(writer.supports(cos.as_ref()));
        assert!(writer.supports(sigmoid.as_ref()));
        // ABS 由 `gurobi_abs` 负责，PWL writer 不得认领。
        // ABS belongs to `gurobi_abs`; the PWL writer must not claim it.
        assert!(!writer.supports(abs.as_ref()));
    }

    /// 构造一个关系指示结构：`s = 2x + 1 - right`，左侧单项式指向列 0。
    /// Build a relation-indicator structure: `s = 2x + 1 - right` with the left monomial on column 0.
    fn indicator_structure(
        kind: crate::symbol::function::InequalityKind,
        big_m: f64,
        right: f64,
    ) -> InequalityStructure<f64> {
        let symbol = Arc::new(crate::symbol::function::InequalityFunction::<f64>::new(
            7_000,
            "ineq_native_plan",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            right,
            kind,
            big_m,
        ));
        InequalityStructure::new("ineq_native_plan", symbol, big_m)
    }

    /// 即时展开的一行在指示列取 `indicator_value` 时对 `s` 的投影：`(关系, 右端项)`
    /// Project one eager row onto `s` for a given indicator value: `(relation, right-hand side)`
    ///
    /// 行里单项式的 `var_index()` 是**列下标**口径（与物化用的 `symbol_to_index` 一致），与
    /// `VariableId::unique_id()` 是两套编号。
    /// A row monomial's `var_index()` uses the **column index** convention (the one materialization's
    /// `symbol_to_index` uses), which is a different numbering from `VariableId::unique_id()`.
    fn eager_indicator_projection(
        constraint: &crate::model::LinearConstraint<f64>,
        indicator_column: usize,
        indicator_value: f64,
    ) -> (ConstraintRelation, f64) {
        let y_coefficient = constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == indicator_column)
            .map(|monomial| *monomial.coefficient())
            .unwrap_or(0.0);
        (
            constraint.inequality.relation,
            constraint.inequality.rhs - y_coefficient * indicator_value,
        )
    }

    #[test]
    fn indicator_plan_admits_relation_kinds_and_reuses_the_eager_tolerance() {
        use crate::symbol::function::InequalityKind;

        for kind in [
            InequalityKind::LessEqual,
            InequalityKind::GreaterEqual,
            InequalityKind::Less,
            InequalityKind::Greater,
        ] {
            let structure = indicator_structure(kind, 8.0, 0.5);
            let plan = plan_indicator_native(&structure)
                .unwrap_or_else(|reason| panic!("{kind:?} should be admitted, got {reason:?}"));

            assert_eq!(plan.name, "ineq_native_plan");
            assert_eq!(plan.result, *structure.result());
            // 条件是 `2x + 1 - 0.5`：单项式用列下标，常数项是即时展开的同一个平移量。
            // The condition is `2x + 1 - 0.5`: monomials use column indices and the constant is the
            // very shift the eager expansion uses.
            assert_eq!(plan.coefficients, vec![(0, 2.0)]);
            assert_eq!(plan.constant, 0.5);
            assert_eq!(plan.big_m, 8.0);
            // 严格性 ε 必须是即时路径的同一常量，而不是本模块自取的值。
            // The strictness ε must be the eager path's very constant, never a value picked here.
            assert_eq!(plan.tolerance, INDICATOR_TOLERANCE);

            let core = indicator_core_relations(kind).expect("admitted kinds expose core relations");
            assert_eq!(plan.when_true, core.when_true);
            assert_eq!(plan.when_false, core.when_false);
            assert_eq!(plan.relaxed_lower_rhs, core.relaxed_lower_rhs);
            assert_eq!(plan.relaxed_upper_rhs, core.relaxed_upper_rhs);
        }
    }

    #[test]
    fn indicator_plan_core_relations_match_the_eager_rows() {
        use crate::symbol::function::InequalityKind;

        // 指示列在列号 1（物化用的 `symbol_to_index` 就是令牌位置）。
        // The indicator column sits at column 1 (materialization's `symbol_to_index` is the token
        // position).
        const INDICATOR_COLUMN: usize = 1;
        // 两个 M 取 2 的幂，使 `(ε - M) + M` 的舍入停留在 ulp 量级；容差 1e-12 仍比 ε = 1e-10 小
        // 两个数量级，足以分辨「同一 ε」与「另一个 ε」。
        // Both M values are powers of two so that `(ε - M) + M` rounds at the ulp scale; the 1e-12
        // tolerance stays two orders of magnitude below ε = 1e-10, which is enough to tell "the same
        // ε" from "another ε".
        let projection_tolerance = 1e-12;

        for kind in [
            InequalityKind::LessEqual,
            InequalityKind::GreaterEqual,
            InequalityKind::Less,
            InequalityKind::Greater,
        ] {
            // 两个结构必须包着**同一个符号**：指示列的 `VariableId` 由符号创建时决定，换一个符号就
            // 换了一套列 ID，`symbol_to_index` 便对不上。
            // Both structures must wrap the **same symbol**: the indicator column's `VariableId` is
            // decided when the symbol is created, and a different symbol means a different set of
            // column IDs that `symbol_to_index` would not match.
            let symbol = Arc::new(crate::symbol::function::InequalityFunction::<f64>::new(
                7_000,
                "ineq_native_plan",
                Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                0.5,
                kind,
                8.0,
            ));
            let small = InequalityStructure::new("ineq_native_plan", symbol.clone(), 8.0);
            let large = InequalityStructure::new("ineq_native_plan", symbol, 16.0);
            let symbol_to_index =
                HashMap::from([(small.result().unique_id() as usize, INDICATOR_COLUMN)]);
            let small_rows = small
                .materialize(&symbol_to_index)
                .expect("the small big-M structure should materialize");
            let large_rows = large
                .materialize(&symbol_to_index)
                .expect("the large big-M structure should materialize");
            let plan = plan_indicator_native(&small).expect("the structure should be admitted");

            for (indicator_value, expected) in
                [(1.0f64, plan.when_true), (0.0f64, plan.when_false)]
            {
                // 核心行（不含 Big-M）的投影不随 M 变化，松弛行的投影按 M 线性变化；原生计划只写
                // 核心行，因此它必须逐位复现「M-不变」的那一条（含严格性 ε）。
                // A core row's projection does not move with M while a relaxed row's moves linearly
                // with it; the native plan writes only the core row, so it must reproduce the
                // M-invariant one bit for bit, including the strictness ε.
                let mut cores = Vec::new();
                for (small_row, large_row) in small_rows.iter().zip(large_rows.iter()) {
                    let small_projection =
                        eager_indicator_projection(small_row, INDICATOR_COLUMN, indicator_value);
                    let large_projection =
                        eager_indicator_projection(large_row, INDICATOR_COLUMN, indicator_value);
                    assert_eq!(
                        small_projection.0, large_projection.0,
                        "a row's relation must not depend on the big-M"
                    );
                    if (small_projection.1 - large_projection.1).abs() <= projection_tolerance {
                        cores.push(small_projection);
                    }
                }

                assert_eq!(
                    cores.len(),
                    1,
                    "exactly one eager row per indicator value must be free of the big-M"
                );
                assert_eq!(cores[0].0, expected.0, "{kind:?} at indicator {indicator_value}");
                assert!(
                    (cores[0].1 - expected.1).abs() <= projection_tolerance,
                    "{kind:?} at indicator {indicator_value}: eager {} vs plan {}",
                    cores[0].1,
                    expected.1
                );
                // 计划里的 ε 与即时行用的是同一常量。
                // The plan's ε is the same constant the eager rows use.
                assert_eq!(plan.tolerance, INDICATOR_TOLERANCE);
            }
        }
    }

    #[test]
    fn indicator_plan_rejects_kinds_and_conditions_it_cannot_write() {
        use crate::symbol::function::{InequalityFunction, InequalityKind};

        // `=` / `!=` 的取假侧是析取，需要 side 辅助列做情形分裂，两条 indicator 表达不了。
        // The false side of `=` / `!=` is a disjunction split by the side helper column, which two
        // indicator constraints cannot express.
        assert!(plan_indicator_native(&indicator_structure(InequalityKind::Equal, 8.0, 0.5)).is_err());
        assert!(
            plan_indicator_native(&indicator_structure(InequalityKind::NotEqual, 8.0, 0.5)).is_err()
        );

        // 非正 Big-M 无法给出任何松弛界。
        // A non-positive big-M cannot provide any relaxation bound.
        assert!(
            plan_indicator_native(&indicator_structure(InequalityKind::LessEqual, 0.0, 0.5)).is_err()
        );

        // 条件里没有任何变量项：在 SDK 里会退化成一条没有变量的空表达式一般约束。
        // A condition without any variable term would degrade into an empty general constraint in the
        // SDK.
        let no_terms = Arc::new(InequalityFunction::<f64>::new(
            7_010,
            "ineq_no_terms",
            Linear::new(Vec::new(), 2.0),
            0.5,
            InequalityKind::LessEqual,
            8.0,
        ));
        let no_terms = InequalityStructure::new("ineq_no_terms", no_terms, 8.0);
        assert!(plan_indicator_native(&no_terms).is_err());

        // 右侧非有限：平移量无法确定。
        // A non-finite right-hand side leaves the shift undetermined.
        let non_finite = Arc::new(InequalityFunction::<f64>::new(
            7_011,
            "ineq_non_finite",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            f64::NAN,
            InequalityKind::LessEqual,
            8.0,
        ));
        let non_finite = InequalityStructure::new("ineq_non_finite", non_finite, 8.0);
        assert!(plan_indicator_native(&non_finite).is_err());
    }

    #[test]
    fn indicator_big_m_proof_accepts_an_implying_big_m_and_rejects_a_short_one() {
        use crate::symbol::function::InequalityKind;

        // `<=` / `>` 需要证明的下侧松弛是 `s >= ε - M`，上侧松弛是 `s <= M`。
        // The lower relaxation `<=` / `>` must prove is `s >= ε - M`, the upper one is `s <= M`.
        let plan = plan_indicator_native(&indicator_structure(InequalityKind::LessEqual, 8.0, 0.5))
            .expect("the structure should be admitted");
        assert!(plan.prove_big_m_relaxation(-1.0, 1.0).is_ok());
        // 盒角上 `s_min = -M` 恰好差一个 ε：Big-M 取自盒的绝对界时必然出现，由证明容差覆盖。
        // At the box corner `s_min = -M` the relaxation misses by exactly one ε: that happens whenever
        // the big-M comes from the box's absolute bound and is covered by the proof's tolerance.
        assert!(plan.prove_big_m_relaxation(-8.0, 8.0).is_ok());
        // 真正不够的 M（相差远超 ε）必须拒绝，否则原生路径会失去即时展开的松弛约束。
        // A genuinely short big-M (missing by far more than ε) must be rejected, otherwise the native
        // path would lose the eager expansion's relaxation.
        assert!(plan.prove_big_m_relaxation(-8.5, 0.0).is_err());
        assert!(plan.prove_big_m_relaxation(0.0, 8.5).is_err());
        // 非有限域无法证明。
        // A non-finite domain cannot be proven.
        assert!(plan.prove_big_m_relaxation(f64::NEG_INFINITY, 1.0).is_err());

        // `>=` / `<` 需要证明的上侧松弛是 `s <= M - ε`。
        // The upper relaxation `>=` / `<` must prove is `s <= M - ε`.
        let plan = plan_indicator_native(&indicator_structure(InequalityKind::Less, 8.0, 0.5))
            .expect("the structure should be admitted");
        assert!(plan.prove_big_m_relaxation(-8.0, 8.0).is_ok());
        assert!(plan.prove_big_m_relaxation(0.0, 8.5).is_err());
    }

    #[test]
    fn indicator_writer_claims_only_relation_indicators() {
        use crate::symbol::function::InequalityKind;

        let writer = GurobiIndicatorWriter::new();
        assert_eq!(writer.name(), "gurobi_indicator");

        let indicator = indicator_structure(InequalityKind::LessEqual, 8.0, 0.5);
        assert!(writer.supports(&indicator));

        let abs: std::sync::Arc<dyn DeferredFunctionStructure<f64>> = std::sync::Arc::new(
            abs_structure(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                VariableId::standalone(8_400),
                VariableId::standalone(8_401),
            ),
        );
        assert!(!writer.supports(abs.as_ref()));
    }

    /// 构造一个 IF-IN 结构：`input = 2x + 1`（x 在列 0）。
    /// Build an IF-IN structure: `input = 2x + 1` with x on column 0.
    fn if_in_structure(values: Vec<f64>, big_m: f64) -> IfInStructure<f64> {
        let symbol = Arc::new(crate::symbol::function::IfInFunction::<f64>::new(
            8_000,
            "ifin_native_plan",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            values,
            big_m,
        ));
        IfInStructure::new("ifin_native_plan", symbol, big_m)
    }

    #[test]
    fn if_in_plan_admits_a_value_set_and_reuses_the_eager_tolerances() {
        use crate::symbol::function::{STRICT_BOUNDARY, STEP_EPSILON};

        let structure = if_in_structure(vec![3.0, 5.0], 8.0);
        let plan = plan_if_in_native(&structure).expect("a two-value set should be admitted");

        assert_eq!(plan.name, "ifin_native_plan");
        assert_eq!(plan.result, *structure.result());
        assert_eq!(plan.coefficients, vec![(0, 2.0)]);
        assert_eq!(plan.input_constant, 1.0);
        assert_eq!(plan.values, vec![3.0, 5.0]);
        assert_eq!(plan.big_m, 8.0);
        // band 容差与严格边界必须是即时路径的同一常量。
        // The band tolerance and strict boundary must be the eager path's very constants.
        assert_eq!(plan.tolerance, STEP_EPSILON);
        assert_eq!(plan.strict_boundary, STRICT_BOUNDARY);
        // 每个候选值一列指示列与一列 side 列，且顺序与值集合一致。
        // Every candidate gets one indicator and one side column, ordered like the value set.
        assert_eq!(plan.indicator_columns, structure.indicator_columns());
        assert_eq!(plan.side_columns, structure.side_columns());
        assert_eq!(plan.indicator_columns.len(), 2);
        assert_eq!(plan.side_columns.len(), 2);
        for column in plan
            .indicator_columns
            .iter()
            .chain(plan.side_columns.iter())
        {
            assert_ne!(column, &plan.result);
        }
    }

    #[test]
    fn if_in_plan_core_relations_match_the_eager_value_rows() {
        const RESULT_COLUMN: usize = 1;
        let projection_tolerance = 1e-12;

        // 两个结构包着同一个符号，只是固定的 Big-M 不同；这样两次物化给出同一组即时行、不同的 M。
        // Both structures wrap the same symbol and differ only in the fixed Big-M, so the two
        // materializations give the same eager rows with different M values.
        let symbol = Arc::new(crate::symbol::function::IfInFunction::<f64>::new(
            8_010,
            "ifin_native_plan",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            vec![3.0, 5.0],
            8.0,
        ));
        let small = IfInStructure::new("ifin_native_plan", symbol.clone(), 8.0);
        let large = IfInStructure::new("ifin_native_plan", symbol, 16.0);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(small.result().unique_id() as usize, RESULT_COLUMN);
        let indicator_columns = small.indicator_columns();
        let side_columns = small.side_columns();
        for (index, column) in indicator_columns.iter().enumerate() {
            symbol_to_index.insert(column.unique_id() as usize, 2 + index);
        }
        for (index, column) in side_columns.iter().enumerate() {
            symbol_to_index.insert(column.unique_id() as usize, 2 + indicator_columns.len() + index);
        }

        let small_rows = small
            .materialize(&symbol_to_index)
            .expect("the small big-M structure should materialize");
        let large_rows = large
            .materialize(&symbol_to_index)
            .expect("the large big-M structure should materialize");
        let plan = plan_if_in_native(&small).expect("the structure should be admitted");

        // 只分析第一个候选值的四行（`{name}_pt0_*`）。
        // Only the first candidate's four rows (`{name}_pt0_*`) are analysed.
        let value_rows: Vec<usize> = small_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.name.starts_with("ifin_native_plan_pt0_"))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(value_rows.len(), 4);

        for (indicator_value, side_value) in [
            (1.0f64, 0.0f64),
            (1.0f64, 1.0f64),
            (0.0f64, 0.0f64),
            (0.0f64, 1.0f64),
        ] {
            let mut core = Vec::new();
            for index in &value_rows {
                let small_projection = eager_indicator_projection_multi(
                    &small_rows[*index],
                    &[(2usize, indicator_value), (4usize, side_value)],
                );
                let large_projection = eager_indicator_projection_multi(
                    &large_rows[*index],
                    &[(2usize, indicator_value), (4usize, side_value)],
                );
                assert_eq!(small_projection.0, large_projection.0);
                if (small_projection.1 - large_projection.1).abs() <= projection_tolerance {
                    core.push(small_projection);
                }
            }

            let expected: Vec<(ConstraintRelation, f64)> = match (indicator_value, side_value) {
                (1.0, _) => vec![
                    (ConstraintRelation::LessEqual, plan.tolerance),
                    (ConstraintRelation::GreaterEqual, -plan.tolerance),
                ],
                (0.0, 0.0) => vec![(ConstraintRelation::LessEqual, -plan.strict_boundary)],
                (0.0, _) => vec![(ConstraintRelation::GreaterEqual, plan.strict_boundary)],
                _ => unreachable!(),
            };
            assert_eq!(core.len(), expected.len(), "unexpected core row count at ({indicator_value}, {side_value})");
            for (relation, rhs) in &expected {
                assert!(
                    core.iter().any(|(core_relation, core_rhs)| {
                        core_relation == relation && (core_rhs - rhs).abs() <= projection_tolerance
                    }),
                    "core relation ({relation:?}, {rhs}) is missing at (b, side) = ({indicator_value}, {side_value}): {core:?}"
                );
            }
        }
    }

    /// 一行在若干「列下标 = 取值」固定下对平移量的投影：`(关系, 右端项)`
    /// Project a row onto the shift while several "column index = value" assignments are fixed:
    /// `(relation, right-hand side)`
    fn eager_indicator_projection_multi(
        constraint: &crate::model::LinearConstraint<f64>,
        assignments: &[(usize, f64)],
    ) -> (ConstraintRelation, f64) {
        let mut rhs = constraint.inequality.rhs;
        for (column, value) in assignments {
            let coefficient = constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .find(|monomial| monomial.var_index() == *column)
                .map(|monomial| *monomial.coefficient())
                .unwrap_or(0.0);
            rhs -= coefficient * value;
        }
        (constraint.inequality.relation, rhs)
    }

    #[test]
    fn if_in_plan_rejects_value_sets_and_inputs_it_cannot_write() {
        use crate::symbol::function::IfInFunction;

        // 空值集合：即时展开退化成 `result = 0`，`or` 一般约束没有零个操作数的含义。
        // An empty value set: eager expansion collapses to `result = 0` and an `or` general constraint
        // has no zero-operand meaning.
        assert!(plan_if_in_native(&if_in_structure(Vec::new(), 8.0)).is_err());

        // 非正 Big-M 无法给出任何松弛界。
        // A non-positive big-M cannot provide any relaxation bound.
        assert!(plan_if_in_native(&if_in_structure(vec![3.0], 0.0)).is_err());

        // 集合值非有限：平移量无法确定。
        // A non-finite set value leaves the shift undetermined.
        assert!(plan_if_in_native(&if_in_structure(vec![f64::NAN], 8.0)).is_err());

        // 输入没有变量项：指示约束的线性表达式会变空。
        // An input without variable terms: the indicator constraints' linear expression would be empty.
        let constant_input = Arc::new(IfInFunction::<f64>::new(
            8_020,
            "ifin_constant_input",
            Linear::new(Vec::new(), 1.0),
            vec![1.0],
            8.0,
        ));
        let constant_input = IfInStructure::new("ifin_constant_input", constant_input, 8.0);
        assert!(plan_if_in_native(&constant_input).is_err());
    }

    #[test]
    fn if_in_big_m_proof_accepts_an_implying_big_m_and_rejects_a_short_one() {
        use crate::symbol::function::STRICT_BOUNDARY;

        let plan = plan_if_in_native(&if_in_structure(vec![3.0], 8.0))
            .expect("the structure should be admitted");
        assert_eq!(plan.strict_boundary, STRICT_BOUNDARY);

        assert!(plan.prove_big_m_relaxation(0, -1.0, 1.0).is_ok());
        // 推断出的 Big-M 是 `max_difference + STRICT_BOUNDARY`，因此盒角上两个条件**恰好取等号**
        // （`M + s_min == sb` 与 `M - s_max == sb`）。这种紧到边界的证明必须通过——它就是真实模型里
        // 出现的情形；容差只用于吸收推断链上的浮点舍入。
        // The inferred Big-M is `max_difference + STRICT_BOUNDARY`, so at a box corner both conditions
        // hold with **equality** (`M + s_min == sb` and `M - s_max == sb`). A proof that tight must
        // pass — it is exactly what a real model produces — and the tolerance only absorbs
        // floating-point rounding along the inference chain.
        assert!(
            plan.prove_big_m_relaxation(
                0,
                plan.strict_boundary - 8.0,
                8.0 - plan.strict_boundary
            )
            .is_ok()
        );
        // 真正不够的 M 必须拒绝，否则原生路径会失去即时展开的松弛约束。
        // A genuinely short big-M must be rejected, otherwise the native path would lose the eager
        // expansion's relaxation.
        assert!(plan.prove_big_m_relaxation(0, -9.0, 0.0).is_err());
        assert!(plan.prove_big_m_relaxation(0, 0.0, 9.0).is_err());
        // 非有限域无法证明。
        // A non-finite domain cannot be proven.
        assert!(plan.prove_big_m_relaxation(0, f64::NEG_INFINITY, 1.0).is_err());
        // 候选值下标越界会被当作无法证明（`shifted_bounds` 依赖同一索引）。
        // An out-of-range candidate index cannot be proven either (`shifted_bounds` uses the same
        // index).
        assert_eq!(plan.shifted_bounds(0.0, 2.0, 0), (-3.0, -1.0));
    }

    #[test]
    fn if_in_writer_claims_only_if_in_structures() {
        let writer = GurobiIfInWriter::new();
        assert_eq!(writer.name(), "gurobi_if_in");
        assert_eq!(GUROBI_IF_IN_SCHEMA, "functions-if-in-1");
        assert_eq!(
            GUROBI_IF_IN_BIG_M_TOLERANCE,
            GUROBI_INDICATOR_BIG_M_TOLERANCE
        );

        let if_in = if_in_structure(vec![3.0], 8.0);
        assert!(writer.supports(&if_in));

        let inequality = indicator_structure(
            crate::symbol::function::InequalityKind::LessEqual,
            8.0,
            0.5,
        );
        assert!(!writer.supports(&inequality));
    }

    /// 构造一个 AND 结构（操作数由调用方给出）。
    /// Build an AND structure with caller-provided operands.
    fn and_structure(polynomials: Vec<Linear<f64>>) -> AndStructure<f64> {
        AndStructure::new(
            "and_native_plan",
            Arc::new(crate::symbol::function::AndFunction::<f64>::new(
                9_000,
                "and_native_plan",
                polynomials,
            )),
            1.0,
        )
    }

    /// 构造一个 OR 结构（操作数由调用方给出）。
    /// Build an OR structure with caller-provided operands.
    fn or_structure(polynomials: Vec<Linear<f64>>) -> OrStructure<f64> {
        OrStructure::new(
            "or_native_plan",
            Arc::new(crate::symbol::function::OrFunction::<f64>::new(
                9_001,
                "or_native_plan",
                polynomials,
            )),
            1.0,
        )
    }

    #[test]
    fn logical_plan_admits_direct_binary_operands_and_keeps_their_order() {
        let operands = vec![
            Linear::new(vec![LinearMonomial::new(1.0, 3)], 0.0),
            Linear::new(vec![LinearMonomial::new(1.0, 7)], 0.0),
        ];

        let and = and_structure(operands.clone());
        let and_plan = plan_and_native(&and).expect("direct binary operands should be admitted");
        assert_eq!(and_plan.name, "and_native_plan");
        assert_eq!(and_plan.result, *and.result());
        // 操作数列必须与即时紧凑 hull 使用的是同一批列，且保持顺序。
        // The operand columns must be exactly the ones the eager compact hull uses, in order.
        assert_eq!(and_plan.operand_indices, vec![3, 7]);
        assert!(and_plan.conjunction);

        let or = or_structure(operands);
        let or_plan = plan_or_native(&or).expect("direct binary operands should be admitted");
        assert_eq!(or_plan.result, *or.result());
        assert_eq!(or_plan.operand_indices, vec![3, 7]);
        assert!(!or_plan.conjunction);

        // 单个操作数也要准入：`result = AND(x)` 就是 `result = x`。
        // A single operand is admitted as well: `result = AND(x)` is `result = x`.
        let single = and_structure(vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)]);
        assert_eq!(
            plan_and_native(&single)
                .expect("a single operand should be admitted")
                .operand_indices,
            vec![0]
        );
    }

    #[test]
    fn logical_plan_rejects_inputs_the_native_relation_cannot_express() {
        // 非单位系数：不是直接二值变量输入，即时展开会走非零指示列分支。
        // A non-unit coefficient is not a direct binary input and eager expansion takes the non-zero
        // indicator branch instead.
        let scaled = and_structure(vec![Linear::new(vec![LinearMonomial::new(2.0, 0)], 0.0)]);
        assert!(plan_and_native(&scaled).is_err());

        // 非零常数项：同样不是直接二值输入。
        // A non-zero constant is not a direct binary input either.
        let shifted = or_structure(vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0)]);
        assert!(plan_or_native(&shifted).is_err());

        // 多个单项式：不是单一列。
        // More than one monomial is not a single column.
        let multi = and_structure(vec![Linear::new(
            vec![
                LinearMonomial::new(1.0, 0),
                LinearMonomial::new(1.0, 1),
            ],
            0.0,
        )]);
        assert!(plan_and_native(&multi).is_err());

        // 空输入集合：没有操作数，SDK 的 and/or 也没有零操作数含义。
        // An empty input set has no operand, and the SDK's and/or has no zero-operand meaning either.
        let empty = and_structure(Vec::new());
        assert!(plan_and_native(&empty).is_err());
    }

    #[test]
    fn logical_writers_claim_only_their_own_structure_kind() {
        assert_eq!(GUROBI_AND_SCHEMA, "functions-and-1");
        assert_eq!(GUROBI_OR_SCHEMA, "functions-or-1");

        let and = and_structure(vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)]);
        let or = or_structure(vec![Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0)]);

        let and_writer = GurobiLogicalWriter::conjunction();
        let or_writer = GurobiLogicalWriter::disjunction();
        assert_eq!(and_writer.name(), "gurobi_and");
        assert_eq!(or_writer.name(), "gurobi_or");
        assert!(and_writer.supports(&and));
        assert!(!and_writer.supports(&or));
        assert!(or_writer.supports(&or));
        assert!(!or_writer.supports(&and));
    }

    /// 构造一个二值化结构：`input = 2x + 1`（x 在列 0），阈值与 Big-M 由调用方给出。
    /// Build a binaryzation structure: `input = 2x + 1` with x on column 0; the threshold and big-M come
    /// from the caller.
    fn binaryzation_structure(
        method: crate::symbol::function::BinaryzationMethod,
        threshold: f64,
        big_m: f64,
        input: Linear<f64>,
    ) -> BinaryzationStructure<f64> {
        let symbol = Arc::new(crate::symbol::function::BinaryzationFunction::<f64>::new(
            9_100,
            "bin_native_plan",
            input,
            threshold,
            big_m,
            method,
        ));
        BinaryzationStructure::new("bin_native_plan", symbol, big_m)
    }

    #[test]
    fn binaryzation_plan_admits_both_variants_and_reuses_the_eager_epsilon() {
        use crate::symbol::function::{BINARYZATION_STRICT_EPSILON, BinaryzationMethod};

        let input = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        for method in [
            BinaryzationMethod::Threshold,
            BinaryzationMethod::BigM,
            BinaryzationMethod::Indicator,
            BinaryzationMethod::SOS1,
        ] {
            let structure = binaryzation_structure(method, 3.0, 8.0, input.clone());
            let plan = plan_binaryzation_native(&structure)
                .unwrap_or_else(|reason| panic!("{method:?} should be admitted, got {reason:?}"));

            assert_eq!(plan.name, "bin_native_plan");
            assert_eq!(plan.result, *structure.result());
            // 条件是 `2x + 1 - 3`：单项式用列下标，常数项是即时展开的同一个平移量。
            // The condition is `2x + 1 - 3`: monomials use column indices and the constant is the very
            // shift the eager expansion uses.
            assert_eq!(plan.coefficients, vec![(0, 2.0)]);
            assert_eq!(plan.constant, -2.0);
            assert_eq!(plan.big_m, 8.0);

            // 核心关系与松弛量与符号文件里的表逐位一致（`Indicator`/`SOS1` 已归一到 BigM）。
            // The core relations and relaxations equal the symbol file's table bit for bit
            // (`Indicator` / `SOS1` are normalised to BigM).
            let expected = binaryzation_core_relations(method);
            assert_eq!(plan.when_true, expected.when_true);
            assert_eq!(plan.when_false, expected.when_false);
            assert_eq!(plan.relaxed_lower_rhs, expected.relaxed_lower_rhs);
            assert_eq!(plan.relaxed_upper_rhs, expected.relaxed_upper_rhs);

            // 严格性 ε 必须是即时路径的同一常量，而不是本模块自取的值。
            // The strictness ε must be the eager path's very constant, never a value picked here.
            match method.mechanism_equivalent() {
                BinaryzationMethod::Threshold => {
                    assert_eq!(plan.when_true, (ConstraintRelation::GreaterEqual, 0.0));
                    assert_eq!(
                        plan.when_false,
                        (
                            ConstraintRelation::LessEqual,
                            -BINARYZATION_STRICT_EPSILON
                        )
                    );
                    assert_eq!(plan.relaxed_lower_rhs, 0.0);
                    assert_eq!(plan.relaxed_upper_rhs, BINARYZATION_STRICT_EPSILON);
                }
                _ => {
                    assert_eq!(
                        plan.when_true,
                        (
                            ConstraintRelation::GreaterEqual,
                            BINARYZATION_STRICT_EPSILON
                        )
                    );
                    assert_eq!(plan.when_false, (ConstraintRelation::LessEqual, 0.0));
                    assert_eq!(plan.relaxed_lower_rhs, BINARYZATION_STRICT_EPSILON);
                    assert_eq!(plan.relaxed_upper_rhs, 0.0);
                }
            }
        }
    }

    #[test]
    fn binaryzation_plan_core_relations_match_the_eager_rows() {
        use crate::symbol::function::BinaryzationMethod;

        const INDICATOR_COLUMN: usize = 1;
        let projection_tolerance = 1e-13;

        for method in [
            BinaryzationMethod::Threshold,
            BinaryzationMethod::BigM,
            BinaryzationMethod::Indicator,
            BinaryzationMethod::SOS1,
        ] {
            // 两个结构包着同一个符号，只有固定的 Big-M 不同；这样两次物化给出同一组即时行、不同的 M。
            // Both structures wrap the same symbol and differ only in the fixed Big-M, so the two
            // materializations give the same eager rows with different M values.
            let symbol = Arc::new(crate::symbol::function::BinaryzationFunction::<f64>::new(
                9_110,
                "bin_native_plan",
                Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
                3.0,
                8.0,
                method,
            ));
            let small = BinaryzationStructure::new("bin_native_plan", symbol.clone(), 8.0);
            let large = BinaryzationStructure::new("bin_native_plan", symbol, 16.0);
            let symbol_to_index =
                HashMap::from([(small.result().unique_id() as usize, INDICATOR_COLUMN)]);

            let small_rows = small
                .materialize(&symbol_to_index)
                .expect("the small big-M structure should materialize");
            let large_rows = large
                .materialize(&symbol_to_index)
                .expect("the large big-M structure should materialize");
            let plan = plan_binaryzation_native(&small).expect("the structure should be admitted");

            for (indicator_value, expected) in
                [(1.0f64, plan.when_true), (0.0f64, plan.when_false)]
            {
                // 核心行的投影不随 M 变化，松弛行按 M 线性变化；原生计划只写核心行，因此它必须逐位
                // 复现「M-不变」的那一条（含严格性 ε）。
                // A core row's projection does not move with M while a relaxed row's moves linearly with
                // it; the native plan writes only the core row, so it must reproduce the M-invariant one
                // bit for bit, including the strictness ε.
                let mut cores = Vec::new();
                for (small_row, large_row) in small_rows.iter().zip(large_rows.iter()) {
                    let small_projection = eager_indicator_projection_multi(
                        small_row,
                        &[(INDICATOR_COLUMN, indicator_value)],
                    );
                    let large_projection = eager_indicator_projection_multi(
                        large_row,
                        &[(INDICATOR_COLUMN, indicator_value)],
                    );
                    assert_eq!(small_projection.0, large_projection.0);
                    if (small_projection.1 - large_projection.1).abs() <= projection_tolerance {
                        cores.push(small_projection);
                    }
                }

                assert_eq!(
                    cores.len(),
                    1,
                    "exactly one eager row per indicator value must be free of the big-M ({method:?})"
                );
                assert_eq!(cores[0].0, expected.0, "{method:?} at y = {indicator_value}");
                assert!(
                    (cores[0].1 - expected.1).abs() <= projection_tolerance,
                    "{method:?} at y = {indicator_value}: eager {} vs plan {}",
                    cores[0].1,
                    expected.1
                );
            }
        }
    }

    #[test]
    fn binaryzation_plan_rejects_inputs_it_cannot_write() {
        use crate::symbol::function::BinaryzationMethod;

        // 输入没有任何变量项：指示约束的线性表达式会变空。
        // An input without variable terms: the indicator constraint's linear expression would be empty.
        let constant_input = binaryzation_structure(
            BinaryzationMethod::Threshold,
            3.0,
            8.0,
            Linear::new(Vec::new(), 1.0),
        );
        assert!(plan_binaryzation_native(&constant_input).is_err());

        // 非正 Big-M 无法给出任何松弛界。
        // A non-positive big-M cannot provide any relaxation bound.
        let zero_big_m = binaryzation_structure(
            BinaryzationMethod::BigM,
            3.0,
            0.0,
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert!(plan_binaryzation_native(&zero_big_m).is_err());

        // 阈值非有限：平移量无法确定。
        // A non-finite threshold leaves the shift undetermined.
        let non_finite_threshold = binaryzation_structure(
            BinaryzationMethod::BigM,
            f64::NAN,
            8.0,
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert!(plan_binaryzation_native(&non_finite_threshold).is_err());

        // 系数非有限：无法写进 SDK 的线性表达式。
        // A non-finite coefficient cannot be written into the SDK's linear expression.
        let non_finite_coefficient = binaryzation_structure(
            BinaryzationMethod::BigM,
            3.0,
            8.0,
            Linear::new(vec![LinearMonomial::new(f64::INFINITY, 0)], 0.0),
        );
        assert!(plan_binaryzation_native(&non_finite_coefficient).is_err());
    }

    #[test]
    fn binaryzation_big_m_proof_accepts_an_implying_big_m_and_rejects_a_short_one() {
        use crate::symbol::function::{BINARYZATION_STRICT_EPSILON, BinaryzationMethod};

        let input = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);

        // 阈值编码需要下侧 `s >= -M`（ε_low = 0）与上侧 `s <= M - ε`（ε_up = ε）。
        // The threshold encoding needs the lower relaxation `s >= -M` (ε_low = 0) and the upper one
        // `s <= M - ε` (ε_up = ε).
        let threshold = plan_binaryzation_native(&binaryzation_structure(
            BinaryzationMethod::Threshold,
            3.0,
            8.0,
            input.clone(),
        ))
        .expect("the threshold structure should be admitted");
        assert!(threshold.prove_big_m_relaxation(-1.0, 1.0).is_ok());
        // 推断出的 Big-M 取 `max(|input - threshold| 盒界, 1)`，因此盒角上恰好取等号
        // （`M - s_max == ε`）；这种紧到边界的证明必须通过。
        // The inferred big-M is `max(|input - threshold| box bound, 1)`, so a box corner gives equality
        // (`M - s_max == ε`); a proof that tight must pass.
        assert!(
            threshold
                .prove_big_m_relaxation(0.0, 8.0 - BINARYZATION_STRICT_EPSILON)
                .is_ok()
        );
        assert!(threshold.prove_big_m_relaxation(0.0, 8.5).is_err());
        assert!(threshold.prove_big_m_relaxation(-8.5, 0.0).is_err());
        assert!(threshold.prove_big_m_relaxation(f64::NEG_INFINITY, 1.0).is_err());

        // Big-M 编码需要下侧 `s >= ε - M`（ε_low = ε）与上侧 `s <= M`（ε_up = 0）。
        // The Big-M encoding needs the lower relaxation `s >= ε - M` (ε_low = ε) and the upper one
        // `s <= M` (ε_up = 0).
        let big_m = plan_binaryzation_native(&binaryzation_structure(
            BinaryzationMethod::BigM,
            3.0,
            8.0,
            input,
        ))
        .expect("the big-M structure should be admitted");
        assert!(big_m.prove_big_m_relaxation(-1.0, 1.0).is_ok());
        assert!(
            big_m
                .prove_big_m_relaxation(BINARYZATION_STRICT_EPSILON - 8.0, 8.0)
                .is_ok()
        );
        assert!(big_m.prove_big_m_relaxation(-8.5, 0.0).is_err());
        assert!(big_m.prove_big_m_relaxation(0.0, 8.5).is_err());
    }

    /// 构造一个蕴含结构：前提与结论都是 `x REL rhs`（x 在列 0），两个子指示器各自冻结一个 Big-M。
    /// Build an implication structure whose premise and consequence are both `x REL rhs` (x on column 0),
    /// with one frozen Big-M per sub-indicator.
    fn imply_structure(
        premise: (ConstraintRelation, f64),
        consequence: (ConstraintRelation, f64),
        big_m: f64,
        premise_big_m: f64,
        consequence_big_m: f64,
    ) -> ImplyStructure<f64> {
        let symbol = Arc::new(crate::symbol::function::ImplyFunction::<f64>::new(
            9_500,
            "imply_native_plan",
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                premise.0,
                premise.1,
            ),
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                consequence.0,
                consequence.1,
            ),
            big_m,
        ));
        let helpers = vec![
            symbol.premise_indicator_variable().id(),
            symbol.consequence_indicator_variable().id(),
        ];
        ImplyStructure::new(
            "imply_native_plan",
            symbol,
            helpers,
            premise_big_m,
            consequence_big_m,
        )
    }

    #[test]
    fn imply_plan_maps_both_sub_indicators_and_reuses_the_batch_one_relations() {
        use crate::symbol::function::indicator_core_relations;

        // 前提 `x >= 1`、结论 `x >= 2`，两个子指示器分别冻结 M = 8 / 12。
        // Premise `x >= 1` and consequence `x >= 2` with the sub-indicators' Big-M frozen at 8 / 12.
        let structure = imply_structure(
            (ConstraintRelation::GreaterEqual, 1.0),
            (ConstraintRelation::GreaterEqual, 2.0),
            10.0,
            8.0,
            12.0,
        );
        // 辅助列非空（两个子指示器的结果列），因此 writer 的辅助列独占性门控是**真实**门控。
        // The helper list is non-empty (both sub-indicators' result columns), so the writer's
        // helper-exclusivity gate is a **real** gate.
        assert_eq!(structure.helpers().len(), 2);

        let plan = plan_imply_native(&structure).expect("the implication should be admitted");
        assert_eq!(plan.name, "imply_native_plan");
        assert_eq!(plan.result, *structure.result());

        for (sub_plan, sub_kind, expected_big_m, expected_name) in [
            (
                &plan.premise,
                structure.premise_indicator().inequality_kind(),
                8.0,
                "imply_native_plan_premise",
            ),
            (
                &plan.consequence,
                structure.consequence_indicator().inequality_kind(),
                12.0,
                "imply_native_plan_consequence",
            ),
        ] {
            assert_eq!(sub_plan.name, expected_name);
            assert_eq!(sub_plan.big_m, expected_big_m);
            // 子指示器的核心关系与 ε 与第 1 批同源：直接来自 `indicator_core_relations`。
            // The sub-indicator's core relations and ε share batch 1's origin: they come straight from
            // `indicator_core_relations`.
            let core =
                indicator_core_relations(sub_kind).expect("only side-free kinds are admitted");
            assert_eq!(sub_plan.when_true, core.when_true);
            assert_eq!(sub_plan.when_false, core.when_false);
            assert_eq!(sub_plan.relaxed_lower_rhs, core.relaxed_lower_rhs);
            assert_eq!(sub_plan.relaxed_upper_rhs, core.relaxed_upper_rhs);
            assert_eq!(sub_plan.tolerance, INDICATOR_TOLERANCE);
        }

        // 两个子指示器的指示列就是它们各自的结果列（关系指示器的结果列即指示列）。
        // Each sub-indicator's indicator column is its own result column (a relation indicator's result
        // column is its indicator column).
        assert_eq!(
            plan.premise.result,
            structure
                .premise_indicator()
                .result_variable()
                .id()
        );
        assert_eq!(
            plan.consequence.result,
            structure
                .consequence_indicator()
                .result_variable()
                .id()
        );
        assert_ne!(plan.premise.result, plan.result);
        assert_ne!(plan.consequence.result, plan.result);
        assert_ne!(plan.premise.result, plan.consequence.result);
    }

    #[test]
    fn imply_plan_rejects_equality_sub_indicators() {
        // `=` 形态的取假侧是析取，需要 side 辅助列做情形分裂：两条指示约束表达不了，整个结构回退。
        // The false side of an `=` form is a disjunction that needs the side helper column for a case
        // split, which two indicator constraints cannot express, so the whole structure falls back.
        let structure = imply_structure(
            (ConstraintRelation::Equal, 1.0),
            (ConstraintRelation::GreaterEqual, 2.0),
            10.0,
            8.0,
            12.0,
        );
        let reason = plan_imply_native(&structure).expect_err("`=` must be rejected");
        match reason {
            FallbackReason::Rejected(message) => assert!(
                message.contains("premise sub-indicator"),
                "the rejection must name the failing sub-indicator: {message}"
            ),
            other => panic!("expected a rejection, got {other:?}"),
        }

        // 结论一侧同样拒绝。
        // The consequence side is rejected the same way.
        let structure = imply_structure(
            (ConstraintRelation::GreaterEqual, 1.0),
            (ConstraintRelation::Equal, 2.0),
            10.0,
            8.0,
            12.0,
        );
        let reason = plan_imply_native(&structure).expect_err("`=` must be rejected");
        match reason {
            FallbackReason::Rejected(message) => assert!(
                message.contains("consequence sub-indicator"),
                "the rejection must name the failing sub-indicator: {message}"
            ),
            other => panic!("expected a rejection, got {other:?}"),
        }
    }

    #[test]
    fn imply_eager_coupling_rows_do_not_depend_on_the_big_m() {
        // 4 条耦合行只含 ±1 系数，因此换一组 Big-M 后必须逐位不变——这就是耦合行不需要任何冗余证明的
        // 机械化依据（与子指示器的松弛行形成对照）。
        // The four coupling rows contain only ±1 coefficients, so they must stay bit-identical under a
        // different pair of Big-M values — the mechanical basis for the coupling needing no redundancy
        // proof at all (in contrast to the sub-indicators' relaxed rows).
        let symbol = Arc::new(crate::symbol::function::ImplyFunction::<f64>::new(
            9_520,
            "imply_native_plan",
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                ConstraintRelation::GreaterEqual,
                1.0,
            ),
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                ConstraintRelation::GreaterEqual,
                2.0,
            ),
            10.0,
        ));
        let helpers = vec![
            symbol.premise_indicator_variable().id(),
            symbol.consequence_indicator_variable().id(),
        ];
        let small = ImplyStructure::new("imply_native_plan", symbol.clone(), helpers.clone(), 8.0, 8.0);
        let large = ImplyStructure::new("imply_native_plan", symbol, helpers, 16.0, 16.0);

        let plan = plan_imply_native(&small).expect("the implication should be admitted");
        let symbol_to_index = HashMap::from([
            (plan.premise.result.unique_id() as usize, 1usize),
            (plan.consequence.result.unique_id() as usize, 2usize),
            (plan.result.unique_id() as usize, 3usize),
        ]);

        let small_rows = small
            .materialize(&symbol_to_index)
            .expect("the small big-M structure should materialize");
        let large_rows = large
            .materialize(&symbol_to_index)
            .expect("the large big-M structure should materialize");
        assert_eq!(small_rows.len(), large_rows.len());

        let small_coupling: Vec<_> = small_rows
            .iter()
            .filter(|row| row.name.contains("_imply_value_"))
            .collect();
        assert_eq!(small_coupling.len(), 4);
        for (small_row, large_row) in small_rows
            .iter()
            .zip(large_rows.iter())
            .filter(|(row, _)| row.name.contains("_imply_value_"))
        {
            assert_eq!(small_row.name, large_row.name);
            // `LinearInequality` 没有实现 `PartialEq`，因此逐字段比较它的结构：单项式、常数项、关系与右端。
            // `LinearInequality` does not implement `PartialEq`, so compare its fields: monomials, constant,
            // relation and right-hand side.
            let shape = |row: &LinearConstraint<f64>| {
                (
                    row.inequality
                        .polynomial
                        .monomials()
                        .iter()
                        .map(|monomial| (monomial.var_index(), *monomial.coefficient()))
                        .collect::<Vec<_>>(),
                    *row.inequality.polynomial.constant_term(),
                    row.inequality.relation,
                    row.inequality.rhs,
                )
            };
            assert_eq!(
                shape(small_row),
                shape(large_row),
                "the coupling row `{}` must not depend on the big-M",
                small_row.name
            );
        }
    }

    #[test]
    fn imply_writer_claims_only_imply_structures() {
        let writer = GurobiImplyWriter::new();
        assert_eq!(writer.name(), "gurobi_imply");
        assert_eq!(GUROBI_IMPLY_SCHEMA, "functions-imply-1");

        let imply = imply_structure(
            (ConstraintRelation::GreaterEqual, 1.0),
            (ConstraintRelation::GreaterEqual, 2.0),
            10.0,
            8.0,
            12.0,
        );
        assert!(writer.supports(&imply));

        let indicator = indicator_structure(InequalityKind::LessEqual, 8.0, 0.5);
        assert!(!writer.supports(&indicator));
    }

    #[test]
    fn binaryzation_writer_claims_only_binaryzation_structures() {
        use crate::symbol::function::BinaryzationMethod;

        let writer = GurobiBinaryzationWriter::new();
        assert_eq!(writer.name(), "gurobi_binaryzation");
        assert_eq!(GUROBI_BINARYZATION_SCHEMA, "functions-binaryzation-1");
        assert_eq!(
            GUROBI_BINARYZATION_BIG_M_TOLERANCE,
            GUROBI_INDICATOR_BIG_M_TOLERANCE
        );

        let binaryzation = binaryzation_structure(
            BinaryzationMethod::BigM,
            3.0,
            8.0,
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert!(writer.supports(&binaryzation));

        let inequality =
            indicator_structure(InequalityKind::LessEqual, 8.0, 0.5);
        assert!(!writer.supports(&inequality));
    }
}
