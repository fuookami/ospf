//! Common 模块 / Common module
use std::error::Error;
use std::sync::Arc;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, MetaModel};
use ospf_rust_core::solver::{FeasibleSolverOutput, SolverCapability, SolverInfo, solvers::GurobiSolver};
use ospf_rust_core::symbol::{BinaryzationMethod, LinearExpressionSymbol, LinearIntermediateSymbol};
use ospf_rust_core::variable::BinaryVariableItem;

use crate::example_modeling::solve_linear_meta_model_typed;

/// 推荐 typed 入口：求解 MetaModel / Recommended typed entry: solve MetaModel
pub fn solve_typed(
    meta_model: MetaModel<f64>,
) -> Result<FeasibleSolverOutput<f64>, Box<dyn Error>> {
    let solver = GurobiSolver::new();
    solve_linear_meta_model_typed(meta_model, &solver)
}

/// 读取解向量中的值，越界时返回 0.0 / Read solution value, returns 0.0 if out of bounds
pub fn read_solution_value(solution: &[f64], idx: usize) -> f64 {
    solution.get(idx).copied().unwrap_or(0.0)
}

/// 注册二进制变量矩阵到模型 / Register binary variable matrix to model
pub fn register_binary_matrix(
    model: &mut MetaModel<f64>,
    rows: usize,
    cols: usize,
    prefix: &str,
) -> Result<Vec<Vec<usize>>, Box<dyn Error>> {
    let mut matrix = vec![vec![0usize; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let var = BinaryVariableItem::auto(&format!("{}_{}_{}", prefix, r, c));
            matrix[r][c] = model.register_variable(var)?;
        }
    }
    Ok(matrix)
}

/// 从 `LinearExpressionSymbol` 提取系数对 `(var_index, coefficient)`
/// Extract coefficient pairs from `LinearExpressionSymbol`
pub fn extract_coeffs(sym: &LinearExpressionSymbol<f64>) -> Vec<(usize, f64)> {
    sym.to_linear_polynomial()
        .monomials()
        .iter()
        .map(|m| (m.var_index(), *m.coefficient()))
        .collect()
}

/// 从 `SymbolCombination` 的单个符号提取系数对
/// Extract coefficient pairs from a single symbol in a `SymbolCombination`
pub fn extract_symbol_coeffs(
    sym: &ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, ospf_rust_multiarray::Shape<1>>,
    index: usize,
) -> Vec<(usize, f64)> {
    extract_coeffs(&sym[index])
}

/// 从索引列表构建线性表达式项 / Build linear expression terms from index list
pub fn linear_expr_from_indices(indices: &[usize], coefficient: f64) -> Vec<(usize, f64)> {
    indices
        .iter()
        .copied()
        .map(|idx| (idx, coefficient))
        .collect()
}

/// 从稀疏项设置线性目标函数 / Set linear objective from sparse terms
pub fn set_linear_objective_from_sparse_terms(
    model: &mut MetaModel<f64>,
    terms: &[(usize, f64)],
    category: ObjectiveCategory,
) {
    let mut objective = vec![0.0; model.num_tokens()];
    for (index, coefficient) in terms {
        objective[*index] = *coefficient;
    }
    model.set_linear_objective(objective, category);
}

/// 添加带元数据的约束 / Add constraint with metadata
pub fn add_constraint_with_metadata(
    model: &mut MetaModel<f64>,
    coefficients: &[(usize, f64)],
    relation: ConstraintRelation,
    rhs: f64,
    name: &str,
    group: Option<Arc<ConstraintGroup>>,
    lazy: bool,
    priority: u32,
    args: Option<String>,
) -> Result<(), Box<dyn Error>> {
    model.add_linear_constraint_with_metadata(
        coefficients,
        relation,
        rhs,
        name,
        group,
        lazy,
        priority,
        args,
    )?;
    Ok(())
}

/// 解析二值化方法，根据求解器能力决定回退策略
/// Resolve binaryzation method, decide fallback strategy based on solver capabilities
pub fn resolve_binaryzation_method(
    solver: &dyn SolverInfo,
    requested: BinaryzationMethod,
) -> (BinaryzationMethod, Option<String>) {
    match requested {
        BinaryzationMethod::Indicator => {
            if solver.supports(SolverCapability::NativeIndicator) {
                (BinaryzationMethod::Indicator, None)
            } else {
                (
                    BinaryzationMethod::BigM,
                    Some(format!(
                        "{} does not support NativeIndicator, fallback to Big-M",
                        solver.name()
                    )),
                )
            }
        }
        BinaryzationMethod::SOS1 => {
            if solver.supports(SolverCapability::NativeSOS1) {
                (BinaryzationMethod::SOS1, None)
            } else {
                (
                    BinaryzationMethod::BigM,
                    Some(format!(
                        "{} does not support NativeSOS1, fallback to Big-M",
                        solver.name()
                    )),
                )
            }
        }
        _ => (requested, None),
    }
}
