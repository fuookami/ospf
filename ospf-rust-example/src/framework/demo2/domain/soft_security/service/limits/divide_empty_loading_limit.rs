use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use crate::framework::demo2::domain::soft_security::aggregation::SoftSecurityAggregation;
use crate::framework::demo2::domain::soft_security::context::SoftSecurityContext;
use crate::framework::demo2::domain::shared::pipeline_mode::mode_name;
use crate::framework::demo2::domain::stowage::model::cargo::CargoCode;

/// 空舱位标志变量索引 / Empty position flag variable indices
///
/// 对齐 Kotlin DivideEmptyLoading model 层，持有三个中间符号。
/// Matches Kotlin DivideEmptyLoading model layer, holds three intermediate symbols.
///
/// - `empty_between_cargo` / `empty_cargo_between_cargo` / `empty_between_empty_cargo`: 按 adjacentPositions 维度
#[derive(Debug, Clone)]
pub struct EmptyFlagVariables {
    /// empty_between_cargo[pair_idx] = IfFunction(loadAmount1 - (loadAmount2 + 1))
    /// loadAmount1 = loadAmountOf(position1) { !Empty }, loadAmount2 = loadAmount[position2]
    pub empty_between_cargo: Vec<usize>,
    /// empty_cargo_between_cargo[pair_idx] = IfFunction((loadAmount1 + loadAmount2) - 2)
    /// loadAmount1 = loadAmountOf(position1) { Empty }, loadAmount2 = loadAmountOf(position2) { !Empty }
    pub empty_cargo_between_cargo: Vec<usize>,
    /// empty_between_empty_cargo[pair_idx] = IfFunction(loadAmount1 - (loadAmount2 + 1))
    /// loadAmount1 = loadAmountOf(position1) { Empty }, loadAmount2 = loadAmount[position2]
    pub empty_between_empty_cargo: Vec<usize>,
}

/// DivideEmptyLoading model 层：注册三个中间符号
/// DivideEmptyLoading model layer: register three intermediate symbols
///
/// 对齐 Kotlin DivideEmptyLoading model，将三个符号注册到模型。
/// Matches Kotlin DivideEmptyLoading model, registers three symbols to model.
pub fn register_divide_empty_loading_symbols(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
) -> Result<EmptyFlagVariables, Box<dyn Error>> {
    let mut next_id = 40000u64;
    let pos_count = context.request.positions.len();
    let adjacent_positions = &context.request.adjacent_positions;
    let pair_count = adjacent_positions.len();

    // 校验 adjacent_positions 索引合法性
    // Validate adjacent_positions indices
    for (i, pair) in adjacent_positions.iter().enumerate() {
        if pair.first >= pos_count {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("adjacent_positions[{}].first ({}) >= position count ({})", i, pair.first, pos_count),
            )));
        }
        if pair.second >= pos_count {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("adjacent_positions[{}].second ({}) >= position count ({})", i, pair.second, pos_count),
            )));
        }
    }

    // 辅助函数：计算 loadAmountOf(position) { filter } 的 Linear 表达式
    // Helper: compute loadAmountOf(position) { filter } as Linear expression
    let load_amount_of = |pos_idx: usize, filter_empty: bool| -> Linear<f64> {
        let monomials: Vec<LinearMonomial<f64>> = (0..context.request.cargos.len())
            .filter(|&c| {
                let is_empty = context.request.cargos[c].code == Some(CargoCode::Empty);
                if filter_empty { is_empty } else { !is_empty }
            })
            .map(|c| LinearMonomial::new(1.0, context.x_idx[c][pos_idx]))
            .collect();
        Linear::new(monomials, 0.0)
    };

    // 辅助函数：计算 loadAmount[position] 的 Linear 表达式（所有 cargo）
    // Helper: compute loadAmount[position] as Linear expression (all cargos)
    let load_amount_all = |pos_idx: usize| -> Linear<f64> {
        let monomials: Vec<LinearMonomial<f64>> = (0..context.request.cargos.len())
            .map(|c| LinearMonomial::new(1.0, context.x_idx[c][pos_idx]))
            .collect();
        Linear::new(monomials, 0.0)
    };

    // 1. 创建 empty_between_cargo[pair_idx] IfFunction 符号
    // Kotlin: emptyBetweenCargo = IfFunction(loadAmount1 - (loadAmount2 + 1))
    // loadAmount1 = loadAmountOf(position1) { !Empty }, loadAmount2 = loadAmount[position2]
    let mut empty_between_cargo_idx = vec![0usize; pair_count];
    for (pair_idx, pair) in adjacent_positions.iter().enumerate() {
        let load_amount1 = load_amount_of(pair.first, false); // non-empty at first
        let load_amount2 = load_amount_all(pair.second);      // all at second

        // condition: loadAmount1 - (loadAmount2 + 1)
        let mut condition_monomials = load_amount1.monomials().to_vec();
        for m in load_amount2.monomials() {
            condition_monomials.push(LinearMonomial::new(-*m.coefficient(), m.var_index()));
        }
        let condition = Linear::new(condition_monomials, *load_amount1.constant_term() - *load_amount2.constant_term() - 1.0);

        let then_expr = Linear::new(Vec::new(), 1.0);  // true branch: 1
        let else_expr = Linear::new(Vec::new(), 0.0);  // false branch: 0

        let if_fn = IfFunction::new(
            next_id,
            &format!("soft_security_empty_between_cargo_{}_{}", mode_name(context.mode), pair_idx),
            condition,
            then_expr,
            else_expr,
        );
        empty_between_cargo_idx[pair_idx] = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        next_id += 1;
    }

    // 2. 创建 empty_cargo_between_cargo[pair_idx] IfFunction 符号
    // Kotlin: emptyCargoBetweenCargo = IfFunction((loadAmount1 + loadAmount2) - 2)
    // loadAmount1 = loadAmountOf(position1) { Empty }, loadAmount2 = loadAmountOf(position2) { !Empty }
    let mut empty_cargo_between_cargo_idx = vec![0usize; pair_count];
    for (pair_idx, pair) in adjacent_positions.iter().enumerate() {
        let load_amount1 = load_amount_of(pair.first, true);  // empty at first
        let load_amount2 = load_amount_of(pair.second, false); // non-empty at second

        // condition: (loadAmount1 + loadAmount2) - 2
        let mut condition_monomials = load_amount1.monomials().to_vec();
        for m in load_amount2.monomials() {
            condition_monomials.push(LinearMonomial::new(*m.coefficient(), m.var_index()));
        }
        let condition = Linear::new(condition_monomials, *load_amount1.constant_term() + *load_amount2.constant_term() - 2.0);

        let then_expr = Linear::new(Vec::new(), 1.0);
        let else_expr = Linear::new(Vec::new(), 0.0);

        let if_fn = IfFunction::new(
            next_id,
            &format!("soft_security_empty_cargo_between_cargo_{}_{}", mode_name(context.mode), pair_idx),
            condition,
            then_expr,
            else_expr,
        );
        empty_cargo_between_cargo_idx[pair_idx] = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        next_id += 1;
    }

    // 3. 创建 empty_between_empty_cargo[pair_idx] IfFunction 符号
    // Kotlin: emptyBetweenEmptyCargo = IfFunction(loadAmount1 - (loadAmount2 + 1))
    // loadAmount1 = loadAmountOf(position1) { Empty }, loadAmount2 = loadAmount[position2]
    let mut empty_between_empty_cargo_idx = vec![0usize; pair_count];
    for (pair_idx, pair) in adjacent_positions.iter().enumerate() {
        let load_amount1 = load_amount_of(pair.first, true); // empty at first
        let load_amount2 = load_amount_all(pair.second);      // all at second

        // condition: loadAmount1 - (loadAmount2 + 1)
        let mut condition_monomials = load_amount1.monomials().to_vec();
        for m in load_amount2.monomials() {
            condition_monomials.push(LinearMonomial::new(-*m.coefficient(), m.var_index()));
        }
        let condition = Linear::new(condition_monomials, *load_amount1.constant_term() - *load_amount2.constant_term() - 1.0);

        let then_expr = Linear::new(Vec::new(), 1.0);
        let else_expr = Linear::new(Vec::new(), 0.0);

        let if_fn = IfFunction::new(
            next_id,
            &format!("soft_security_empty_between_empty_cargo_{}_{}", mode_name(context.mode), pair_idx),
            condition,
            then_expr,
            else_expr,
        );
        empty_between_empty_cargo_idx[pair_idx] = if_fn.result_variable().index();
        model.add_symbol(Arc::new(if_fn))?;
        next_id += 1;
    }

    Ok(EmptyFlagVariables {
        empty_between_cargo: empty_between_cargo_idx,
        empty_cargo_between_cargo: empty_cargo_between_cargo_idx,
        empty_between_empty_cargo: empty_between_empty_cargo_idx,
    })
}

/// DivideEmptyLoading limit 层：添加 minimize 目标
/// DivideEmptyLoading limit layer: add minimize objective
///
/// 对齐 Kotlin DivideEmptyLoadingLimit，对三个符号添加 minimize 目标。
/// Matches Kotlin DivideEmptyLoadingLimit, adds minimize objective for three symbols.
pub fn apply_divide_empty_loading_objective(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    let mode = context.mode;

    // 从 aggregation 获取 EmptyFlagVariables（如果已注册）
    let empty_flag_vars = match &aggregation.empty_flag_variables {
        Some(vars) => vars,
        None => return Ok(()), // 未注册则跳过
    };

    // 添加 minimize 目标（按 adjacentPositions 维度遍历）
    let pair_count = empty_flag_vars.empty_between_cargo.len();
    let mut objective_terms: Vec<(usize, f64)> = Vec::new();
    for i in 0..pair_count {
        if empty_flag_vars.empty_between_cargo[i] > 0 {
            objective_terms.push((empty_flag_vars.empty_between_cargo[i], 1.0));
        }
        if empty_flag_vars.empty_cargo_between_cargo[i] > 0 {
            objective_terms.push((empty_flag_vars.empty_cargo_between_cargo[i], 1.0));
        }
        if empty_flag_vars.empty_between_empty_cargo[i] > 0 {
            objective_terms.push((empty_flag_vars.empty_between_empty_cargo[i], 1.0));
        }
    }
    if !objective_terms.is_empty() {
        let obj_input = ospf_rust_core::model::LinearObjectiveInput::minimize(
            &format!("soft_security_divide_empty_{}", mode_name(mode)),
        )
        .terms(objective_terms.iter().copied());
        model.add_linear_objective_input(obj_input);
    }

    Ok(())
}

/// 分离空装载限制: requires_separation 货物应分散装载
/// 对齐 Kotlin DivideEmptyLoadingLimit
///
/// 创建 emptyFlag[i] IfFunction 符号，并使用分离约束限制每个舱位最多一个分离货物。
/// 创建 empty_between_cargo、empty_cargo_between_cargo、empty_between_empty_cargo 三个符号。
///
/// 注意：此函数已重构为 model + limit 两层模式。
/// 请使用 `register_divide_empty_loading_symbols` 和 `apply_divide_empty_loading_objective`。
pub fn apply_divide_empty_loading_limits(
    model: &mut MetaModel<f64>,
    context: &SoftSecurityContext<'_>,
    aggregation: &mut SoftSecurityAggregation,
) -> Result<(), Box<dyn Error>> {
    // Model 层：注册三个中间符号
    let empty_flag_vars = register_divide_empty_loading_symbols(model, context)?;

    // 存储到 aggregation 供 limit 层使用
    aggregation.empty_flag_variables = Some(empty_flag_vars);

    // Limit 层：添加 minimize 目标
    apply_divide_empty_loading_objective(model, context, aggregation)?;

    Ok(())
}
