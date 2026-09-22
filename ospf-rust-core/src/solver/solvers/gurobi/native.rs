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
use crate::model::intermediate::{
    DeferredFunctionStructure, FallbackReason, NativeFunctionWriter, NativeWriteOutcome,
    NativeWriteRecord, NativeWriteRequest,
};
use crate::symbol::function::AbsStructure;
use crate::variable::VariableId;
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
/// 只包含求解器无关的数据：函数名与需要写入的两列。
/// Contains solver-neutral data only: the function name and the two columns to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsNativePlan {
    /// 函数名称，用作原生约束名称 / Function name used as the native constraint name
    pub name: String,
    /// 结果列 `y` / Result column `y`
    pub result: VariableId,
    /// 参数列 `x`，满足 `y = |x|` / Argument column `x` with `y = |x|`
    pub argument: VariableId,
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
    Ok(AbsNativePlan {
        name: structure.function_name().to_string(),
        result: structure.result().clone(),
        argument: structure.side().clone(),
    })
}

/// 求解器容器：Gurobi 模型与"列 ID -> SDK 变量"的映射。
///
/// 变量在 native writer 运行之前已按中间模型的列顺序创建，因此容器的职责只是提供查询入口，
/// 而不是再次创建列——这样原生路径不会新增或删除任何公开列。
///
/// Solver container: the Gurobi model plus the "column ID -> SDK variable" mapping.
///
/// Variables are created from the intermediate model's column order before native writers run, so
/// the container only offers lookups instead of creating columns again; the native path therefore
/// never adds or removes a public column.
pub struct GurobiNativeContainer<'a> {
    /// Gurobi 模型 / Gurobi model
    pub model: &'a mut Model,
    /// 列 ID 到 SDK 变量的映射 / Mapping from column ID to SDK variable
    pub columns: HashMap<VariableId, Var>,
}

impl Debug for GurobiNativeContainer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GurobiNativeContainer")
            .field("columns", &self.columns.len())
            .finish()
    }
}

impl<'a> GurobiNativeContainer<'a> {
    /// 创建容器 / Create a container.
    pub fn new(model: &'a mut Model, columns: HashMap<VariableId, Var>) -> Self {
        Self { model, columns }
    }

    /// 按列 ID 查找 SDK 变量 / Look up the SDK variable of a column ID.
    pub fn variable(&self, id: &VariableId) -> Option<Var> {
        self.columns.get(id).copied()
    }
}

/// Gurobi 的 ABS 原生 writer / Gurobi's native ABS writer.
///
/// 接入方式：把本 writer 注册进 `NativeFunctionWriterRegistry<GurobiNativeContainer<'_>, f64>`，
/// 由 `MechanismModel::lower_deferred_functions` 统一调度。准入失败返回 `Fallback`，SDK 写入
/// 失败返回 `Err`，由调用方对整模型回退——与 Kotlin 的"写入失败丢弃整模型并重建 fallback"
/// 一致。
///
/// How to use it: register this writer in a
/// `NativeFunctionWriterRegistry<GurobiNativeContainer<'_>, f64>` and let
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

impl NativeFunctionWriter<GurobiNativeContainer<'_>, f64> for GurobiAbsWriter {
    fn name(&self) -> &str {
        "gurobi_abs"
    }

    fn supports(&self, structure: &dyn DeferredFunctionStructure<f64>) -> bool {
        structure.as_any().downcast_ref::<AbsStructure<f64>>().is_some()
    }

    fn write_batch(
        &self,
        container: &mut GurobiNativeContainer<'_>,
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

            let (Some(result_var), Some(argument_var)) = (
                container.variable(&plan.result),
                container.variable(&plan.argument),
            ) else {
                outcomes.push(NativeWriteOutcome::Fallback(FallbackReason::Rejected(format!(
                    "abs `{}` native lowering could not find both columns in the Gurobi model",
                    plan.name
                ))));
                continue;
            };

            // SDK 写入失败直接返回错误：调用方必须对整模型回退，而不是保留半写入状态。
            // An SDK write failure returns an error: the caller must fall back for the whole model
            // instead of keeping a half-written state.
            container
                .model
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::flatten::{Linear, LinearMonomial};
    use crate::symbol::function::AbsFunction;
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
        assert_eq!(plan.argument, argument);
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
}
