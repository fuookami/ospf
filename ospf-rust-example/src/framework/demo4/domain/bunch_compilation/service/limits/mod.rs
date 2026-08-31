//! 编组编制约束限制 / Bunch compilation constraint limits.
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};

/// 机队平衡限制 / Fleet balance limit
/// 对齐 Kotlin FleetBalanceLimit
///
/// 对每个机队平衡约束：
/// 1. 添加约束: slack >= min_balance
/// 2. 添加 minimize 目标: sum(coefficient * slack)
pub fn apply_fleet_balance_limit(
    model: &mut MetaModel<f64>,
    _compilations: &[super::super::model::Compilation],
    fleet_balances: &[super::super::model::FleetBalance],
) -> Result<(), Box<dyn Error>> {
    for balance in fleet_balances {
        // 获取松弛变量索引
        let slack_indices = balance.slack_indices();

        // 对每个 limit 添加约束: slack >= min_balance
        for (l, limit) in balance.limits.iter().enumerate() {
            let slack_idx = slack_indices[l];
            model.add_linear_constraint(
                &[(slack_idx, 1.0)],
                ConstraintRelation::GreaterEqual,
                limit.min_balance as f64,
                &format!("fleet_balance_{}_{}", balance.aircraft_type, l),
            )?;
        }

        // 添加 minimize 目标: sum(slack)
        let objective_terms: Vec<(usize, f64)> = slack_indices.iter()
            .map(|&idx| (idx, 1.0))
            .collect();
        if !objective_terms.is_empty() {
            model.add_linear_objective(&objective_terms, &format!("fleet_balance_{}", balance.aircraft_type));
        }
    }

    Ok(())
}

/// 航班链接限制 / Flight link limit
/// 对齐 Kotlin FlightLinkLimit
///
/// 对每个航班链接：
/// 1. 添加约束: slack >= 1
/// 2. 添加 minimize 目标: sum(slack)
pub fn apply_flight_link_limit(
    model: &mut MetaModel<f64>,
    _compilations: &[super::super::model::Compilation],
    flight_links: &[super::super::model::FlightLink],
) -> Result<(), Box<dyn Error>> {
    let mut next_id = 50000u64;
    let mut objective_terms = Vec::new();

    for link in flight_links {
        // 注册松弛变量
        let slack_symbol = ospf_rust_core::symbol::LinearExpressionSymbol::new(
            next_id,
            &format!("flight_link_slack_{}_{}", link.from_flight, link.to_flight),
            Vec::new(),
            0.0,
        );
        let slack_idx = next_id as usize;
        model.add_symbol(Arc::new(slack_symbol))?;
        next_id += 1;

        // 添加约束: slack >= 1
        model.add_linear_constraint(
            &[(slack_idx, 1.0)],
            ConstraintRelation::GreaterEqual,
            1.0,
            &format!("flight_link_{}_{}", link.from_flight, link.to_flight),
        )?;

        objective_terms.push((slack_idx, 1.0));
    }

    // 添加 minimize 目标
    if !objective_terms.is_empty() {
        model.add_linear_objective(&objective_terms, "flight_link");
    }

    Ok(())
}
