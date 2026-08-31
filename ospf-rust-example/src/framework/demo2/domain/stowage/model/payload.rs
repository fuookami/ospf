//! 载荷模型 / Payload model
use super::super::super::shared::units;
use super::item::Item;
use super::load::LoadVariables;
use super::position::Position;
use super::stowage::{StowageMode, StowageVariables};
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::Unit;
use std::error::Error;
use std::sync::Arc;

/// 载荷变量索引 / Payload variable indices (对齐 Kotlin Payload)
#[derive(Debug, Clone)]
pub struct PayloadVariables {
    /// 主舱估算载荷 / Main deck estimate payload
    pub main_estimate_payload: usize,
    /// 下舱估算载荷 / Lower deck estimate payload
    pub low_estimate_payload: usize,
    /// 总估算载荷 / Total estimate payload
    pub estimate_payload: usize,
    /// 主舱实际载荷 / Main deck actual payload
    pub main_actual_payload: usize,
    /// 下舱实际载荷 / Lower deck actual payload
    pub low_actual_payload: usize,
    /// 总实际载荷 / Total actual payload
    pub actual_payload: usize,
}

/// 载荷模型 / Payload model (对齐 Kotlin Payload)
///
/// Creates 6 intermediate symbols for payload by deck and estimation type:
/// - mainEstimatePayload: main deck estimate (estimateLoadWeight or constant for FullLoad)
/// - lowEstimatePayload: lower deck estimate
/// - estimatePayload: total estimate
/// - mainActualPayload: main deck actual (actualLoadWeight or constant for FullLoad)
/// - lowActualPayload: lower deck actual
/// - actualPayload: total actual
#[derive(Debug)]
pub struct Payload {
    /// 计划载荷 / Planned payload
    pub planned_payload: Quantity<f64, Unit>,
    /// 最大载荷 / Maximum payload
    pub max_payload: Quantity<f64, Unit>,
    /// 计算载荷 / Computed payload
    pub computed_payload: Option<Quantity<f64, Unit>>,
    /// 物品列表 / Item list
    pub items: Vec<Item>,
    /// 舱位列表 / Position list
    pub positions: Vec<Position>,
}

impl Payload {
    /// 注册载荷中间符号到模型 / Register payload intermediate symbols into the model
    ///
    /// 对齐 Kotlin Payload.register:
    /// - FullLoad: constant sums of item weights by deck location
    /// - Predistribution: mainEstimate/lowEstimate from estimateLoadWeight by deck,
    ///   estimatePayload from computed/planned payload constant
    /// - WeightRecommendation: all from load weight variables by deck
    pub fn register(
        &self,
        stowage_mode: StowageMode,
        model: &mut MetaModel<f64>,
        _stowage_vars: &StowageVariables,
        load_vars: &LoadVariables,
    ) -> Result<PayloadVariables, Box<dyn Error>> {
        let position_count = self.positions.len();
        let mut next_id = 40000u64;
        let wu = units::weight_unit();

        match stowage_mode {
            StowageMode::FullLoad => {
                // FullLoad: payload is constant (sum of item weights by deck)
                // Aligned with Kotlin: items.fold(Flt64.zero) { acc, item -> if (enabled) acc + weight else acc }

                // mainEstimatePayload = sum(item.weight for items with main location)
                let main_estimate_constant: f64 = self
                    .items
                    .iter()
                    .filter(|item| item.location.main())
                    .map(|item| units::quantity_value_in_unit(&item.weight, &wu).unwrap_or(0.0))
                    .sum();
                let main_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_estimate_payload",
                    Vec::new(),
                    main_estimate_constant,
                );
                model.add_symbol(Arc::new(main_estimate_symbol))?;
                let main_estimate_payload = next_id as usize;
                next_id += 1;

                // lowEstimatePayload = sum(item.weight for items with low location)
                let low_estimate_constant: f64 = self
                    .items
                    .iter()
                    .filter(|item| item.location.low())
                    .map(|item| units::quantity_value_in_unit(&item.weight, &wu).unwrap_or(0.0))
                    .sum();
                let low_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_estimate_payload",
                    Vec::new(),
                    low_estimate_constant,
                );
                model.add_symbol(Arc::new(low_estimate_symbol))?;
                let low_estimate_payload = next_id as usize;
                next_id += 1;

                // estimatePayload = sum(item.weight for all items)
                let estimate_constant: f64 = self
                    .items
                    .iter()
                    .map(|item| units::quantity_value_in_unit(&item.weight, &wu).unwrap_or(0.0))
                    .sum();
                let estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "estimate_payload",
                    Vec::new(),
                    estimate_constant,
                );
                model.add_symbol(Arc::new(estimate_symbol))?;
                let estimate_payload = next_id as usize;
                next_id += 1;

                // In FullLoad, actual = estimate (items are fully determined)
                let main_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_actual_payload",
                    Vec::new(),
                    main_estimate_constant,
                );
                model.add_symbol(Arc::new(main_actual_symbol))?;
                let main_actual_payload = next_id as usize;
                next_id += 1;

                let low_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_actual_payload",
                    Vec::new(),
                    low_estimate_constant,
                );
                model.add_symbol(Arc::new(low_actual_symbol))?;
                let low_actual_payload = next_id as usize;
                next_id += 1;

                let actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "actual_payload",
                    Vec::new(),
                    estimate_constant,
                );
                model.add_symbol(Arc::new(actual_symbol))?;
                let actual_payload = next_id as usize;

                Ok(PayloadVariables {
                    main_estimate_payload,
                    low_estimate_payload,
                    estimate_payload,
                    main_actual_payload,
                    low_actual_payload,
                    actual_payload,
                })
            }

            StowageMode::Predistribution => {
                // mainEstimatePayload = sum(estimateLoadWeight[j] for Main deck positions)
                let mut main_estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_main_deck {
                        main_estimate_monomials
                            .push(LinearMonomial::new(1.0, load_vars.estimate_load_weight[j]));
                    }
                }
                let main_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_estimate_payload",
                    main_estimate_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(main_estimate_symbol))?;
                let main_estimate_payload = next_id as usize;
                next_id += 1;

                // lowEstimatePayload = sum(estimateLoadWeight[j] for Low deck positions)
                let mut low_estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_low_deck {
                        low_estimate_monomials
                            .push(LinearMonomial::new(1.0, load_vars.estimate_load_weight[j]));
                    }
                }
                let low_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_estimate_payload",
                    low_estimate_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(low_estimate_symbol))?;
                let low_estimate_payload = next_id as usize;
                next_id += 1;

                // estimatePayload = computedPayload or plannedPayload (constant)
                // Aligned with Kotlin: (computedPayload ?: plannedPayload).to(weightUnit).value
                let estimate_value = match &self.computed_payload {
                    Some(cp) => units::quantity_value_in_unit(cp, &wu).unwrap_or(0.0),
                    None => {
                        units::quantity_value_in_unit(&self.planned_payload, &wu).unwrap_or(0.0)
                    }
                };
                let estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "estimate_payload",
                    Vec::new(),
                    estimate_value,
                );
                model.add_symbol(Arc::new(estimate_symbol))?;
                let estimate_payload = next_id as usize;
                next_id += 1;

                // mainActualPayload = sum(actualLoadWeight[j] for Main deck positions)
                let mut main_actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_main_deck {
                        main_actual_monomials
                            .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                    }
                }
                let main_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_actual_payload",
                    main_actual_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(main_actual_symbol))?;
                let main_actual_payload = next_id as usize;
                next_id += 1;

                // lowActualPayload = sum(actualLoadWeight[j] for Low deck positions)
                let mut low_actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_low_deck {
                        low_actual_monomials
                            .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                    }
                }
                let low_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_actual_payload",
                    low_actual_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(low_actual_symbol))?;
                let low_actual_payload = next_id as usize;
                next_id += 1;

                // actualPayload = sum(actualLoadWeight[j] for all positions)
                let mut actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for j in 0..position_count {
                    actual_monomials
                        .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                }
                let actual_symbol =
                    LinearExpressionSymbol::new(next_id, "actual_payload", actual_monomials, 0.0);
                model.add_symbol(Arc::new(actual_symbol))?;
                let actual_payload = next_id as usize;

                Ok(PayloadVariables {
                    main_estimate_payload,
                    low_estimate_payload,
                    estimate_payload,
                    main_actual_payload,
                    low_actual_payload,
                    actual_payload,
                })
            }

            StowageMode::WeightRecommendation => {
                // mainEstimatePayload = sum(estimateLoadWeight[j] for Main deck positions)
                let mut main_estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_main_deck {
                        main_estimate_monomials
                            .push(LinearMonomial::new(1.0, load_vars.estimate_load_weight[j]));
                    }
                }
                let main_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_estimate_payload",
                    main_estimate_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(main_estimate_symbol))?;
                let main_estimate_payload = next_id as usize;
                next_id += 1;

                // lowEstimatePayload = sum(estimateLoadWeight[j] for Low deck positions)
                let mut low_estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_low_deck {
                        low_estimate_monomials
                            .push(LinearMonomial::new(1.0, load_vars.estimate_load_weight[j]));
                    }
                }
                let low_estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_estimate_payload",
                    low_estimate_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(low_estimate_symbol))?;
                let low_estimate_payload = next_id as usize;
                next_id += 1;

                // estimatePayload = sum(estimateLoadWeight[j] for all positions)
                // Aligned with Kotlin WeightRecommendation: sum over all positions
                let mut estimate_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for j in 0..position_count {
                    estimate_monomials
                        .push(LinearMonomial::new(1.0, load_vars.estimate_load_weight[j]));
                }
                let estimate_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "estimate_payload",
                    estimate_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(estimate_symbol))?;
                let estimate_payload = next_id as usize;
                next_id += 1;

                // mainActualPayload = sum(actualLoadWeight[j] for Main deck positions)
                let mut main_actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_main_deck {
                        main_actual_monomials
                            .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                    }
                }
                let main_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "main_actual_payload",
                    main_actual_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(main_actual_symbol))?;
                let main_actual_payload = next_id as usize;
                next_id += 1;

                // lowActualPayload = sum(actualLoadWeight[j] for Low deck positions)
                let mut low_actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for (j, position) in self.positions.iter().enumerate() {
                    if position.is_low_deck {
                        low_actual_monomials
                            .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                    }
                }
                let low_actual_symbol = LinearExpressionSymbol::new(
                    next_id,
                    "low_actual_payload",
                    low_actual_monomials,
                    0.0,
                );
                model.add_symbol(Arc::new(low_actual_symbol))?;
                let low_actual_payload = next_id as usize;
                next_id += 1;

                // actualPayload = sum(actualLoadWeight[j] for all positions)
                let mut actual_monomials: Vec<LinearMonomial<f64>> = Vec::new();
                for j in 0..position_count {
                    actual_monomials
                        .push(LinearMonomial::new(1.0, load_vars.actual_load_weight[j]));
                }
                let actual_symbol =
                    LinearExpressionSymbol::new(next_id, "actual_payload", actual_monomials, 0.0);
                model.add_symbol(Arc::new(actual_symbol))?;
                let actual_payload = next_id as usize;

                Ok(PayloadVariables {
                    main_estimate_payload,
                    low_estimate_payload,
                    estimate_payload,
                    main_actual_payload,
                    low_actual_payload,
                    actual_payload,
                })
            }
        }
    }
}
