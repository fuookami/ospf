//! Demo15 模块 / Demo15 module
use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_core::variable::{UInteger, VariableCombination1D, VariableCombination3D, VariableRange};

use super::common::{read_solution_value, solve_typed};

/// 替换规则数据结构 / Replacement rule data structure
#[derive(Clone, Copy)]
struct Replacement {
    /// 源车型索引 / Source car model index
    from: usize,
    /// 目标车型索引 / Target car model index
    to: usize,
    /// 最大替换比例 / Maximum replacement ratio
    max_ratio: f64,
}

/// 车型数据结构 / Car model data structure
#[derive(Clone)]
struct CarModel {
    /// 车型名称 / Car model name
    name: String,
}

impl CarModel {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// 配送中心数据结构 / Distribution center data structure
#[derive(Clone)]
struct Center {
    /// 中心名称 / Center name
    name: String,
    /// 各车型需求量 / Demands for each car model
    demands: Vec<f64>,
    /// 替换规则列表 / Replacement rules
    replacements: Vec<Replacement>,
}

impl Center {
    fn new(name: &str, demands: Vec<f64>, replacements: Vec<Replacement>) -> Self {
        Self {
            name: name.to_string(),
            demands,
            replacements,
        }
    }
}

/// 制造商数据结构 / Manufacturer data structure
#[derive(Clone)]
struct Manufacturer {
    /// 制造商名称 / Manufacturer name
    name: String,
    /// 各车型产能 / Productivity for each car model
    productivity_by_model: Vec<Option<f64>>,
    /// 到各配送中心的物流成本 / Logistics cost to each center
    logistics_cost_to_centers: Vec<f64>,
}

impl Manufacturer {
    fn new(
        name: &str,
        productivity_by_model: Vec<Option<f64>>,
        logistics_cost_to_centers: Vec<f64>,
    ) -> Self {
        Self {
            name: name.to_string(),
            productivity_by_model,
            logistics_cost_to_centers,
        }
    }
}

/// 构建车型列表 / Build car model list
fn build_car_models() -> Vec<CarModel> {
    vec![
        CarModel::new("M1"),
        CarModel::new("M2"),
        CarModel::new("M3"),
        CarModel::new("M4"),
    ]
}

/// 构建配送中心列表 / Build distribution center list
fn build_centers() -> Vec<Center> {
    vec![
        Center::new(
            "Denver",
            vec![700.0, 500.0, 500.0, 600.0],
            vec![
                Replacement {
                    from: 0,
                    to: 1,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 0,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 2,
                    to: 3,
                    max_ratio: 0.20,
                },
                Replacement {
                    from: 3,
                    to: 2,
                    max_ratio: 0.20,
                },
            ],
        ),
        Center::new(
            "Miami",
            vec![600.0, 500.0, 200.0, 100.0],
            vec![
                Replacement {
                    from: 0,
                    to: 1,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 0,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 3,
                    max_ratio: 0.05,
                },
                Replacement {
                    from: 3,
                    to: 1,
                    max_ratio: 0.05,
                },
            ],
        ),
    ]
}

/// 构建制造商列表 / Build manufacturer list
fn build_manufacturers() -> Vec<Manufacturer> {
    vec![
        Manufacturer::new(
            "LosAngeles",
            vec![None, None, Some(700.0), Some(300.0)],
            vec![80.0, 215.0],
        ),
        Manufacturer::new(
            "Detroit",
            vec![Some(500.0), Some(600.0), None, Some(400.0)],
            vec![100.0, 108.0],
        ),
        Manufacturer::new(
            "NewOrleans",
            vec![Some(800.0), Some(400.0), None, None],
            vec![102.0, 68.0],
        ),
    ]
}

/// 多车型配送问题模型 / Multi-model vehicle distribution problem model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct DistributionModel {
    /// 配送量决策变量 / Shipment decision variables
    x_vars: VariableCombination3D<UInteger>,
    /// 配送量模型索引数组 / Shipment model index array
    x_idx: MultiArray<usize, Shape<3>>,
    /// 替换比例变量 / Replacement ratio variables (per center)
    y_vars: Vec<VariableCombination1D<UInteger>>,
    /// 替换比例模型索引数组 / Replacement ratio model index array (per center)
    y_idx: Vec<MultiArray<usize, Shape<1>>>,
    /// 成本符号 / Cost symbol
    cost: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 运输量符号 (manufacturer x car_model) / Transport symbol
    trans: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
    /// 接收量符号 (center x car_model) / Receive symbol
    receive: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
    /// 需求替换符号 (center x car_model) / Demand replacement symbol
    demand: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
}

impl DistributionModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        manufacturers: &[Manufacturer],
        centers: &[Center],
        car_models: &[CarModel],
    ) -> Result<Self, Box<dyn Error>> {
        // 1. 注册 x 变量组合 (manufacturer x center x car_model)
        let x_shape = Shape::new([manufacturers.len(), centers.len(), car_models.len()]);
        let x_vars: VariableCombination3D<UInteger> =
            VariableCombination3D::with_name_and_range_generator(
                x_shape.clone(),
                "x",
                |_index, vector| format!("{}_{}_{}", vector[0], vector[1], vector[2]),
                |_index, vector| {
                    if manufacturers[vector[0]].productivity_by_model[vector[2]].is_some() {
                        VariableRange::with_lower(0.0)
                    } else {
                        VariableRange::fixed(0.0)
                    }
                },
            );
        let x_idx = model.register_combination(&x_vars)?;

        // 2. 注册 y 变量组合 (per center replacement ratios)
        let mut y_vars: Vec<VariableCombination1D<UInteger>> = Vec::with_capacity(centers.len());
        let mut y_idx: Vec<MultiArray<usize, Shape<1>>> = Vec::with_capacity(centers.len());
        for d in 0..centers.len() {
            let y_shape = Shape::new([centers[d].replacements.len()]);
            let y_for_center: VariableCombination1D<UInteger> =
                VariableCombination1D::with_name_and_range_generator(
                    y_shape,
                    &format!("y_{}", d),
                    |_index, vector| vector[0].to_string(),
                    |_index, vector| {
                        VariableRange::bounded(0.0, centers[d].replacements[vector[0]].max_ratio)
                    },
                );
            let y_indices = model.register_combination(&y_for_center)?;
            y_vars.push(y_for_center);
            y_idx.push(y_indices);
        }

        // 3. 构建成本符号
        let mfrs = manufacturers;
        let x_idx_ref = &x_idx;
        let cost: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>> = SymbolCombination::new(Shape::new([1]), "cost", |_idx, _vec| {
            let mut terms = Vec::new();
            for m in 0..mfrs.len() {
                for d in 0..centers.len() {
                    for c in 0..car_models.len() {
                        terms.push(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            mfrs[m].logistics_cost_to_centers[d],
                            x_idx_ref[&[m, d, c]],
                        ));
                    }
                }
            }
            let poly = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
            let id = ospf_rust_core::symbol::next_auto_intermediate_symbol_id();
            LinearExpressionSymbol::new(id, "total_cost", poly.monomials().to_vec(), *poly.constant_term())
        });
        model.add_symbol_combination(&cost)?;

        // 4. 构建运输量符号 (manufacturer x car_model)
        let trans: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>> = SymbolCombination::new(
            Shape::new([manufacturers.len(), car_models.len()]),
            "trans",
            |_idx, vec| {
                let m = vec[0];
                let c = vec[1];
                let terms: Vec<_> = (0..centers.len())
                    .map(|d| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx_ref[&[m, d, c]]))
                    .collect();
                let poly = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
                let id = ospf_rust_core::symbol::next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(id, &format!("trans_{}_{}", m, c), poly.monomials().to_vec(), *poly.constant_term())
            },
        );
        model.add_symbol_combination(&trans)?;

        // 5. 构建接收量符号 (center x car_model)
        let receive: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>> = SymbolCombination::new(
            Shape::new([centers.len(), car_models.len()]),
            "receive",
            |_idx, vec| {
                let d = vec[0];
                let c = vec[1];
                let terms: Vec<_> = (0..manufacturers.len())
                    .map(|m| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx_ref[&[m, d, c]]))
                    .collect();
                let poly = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
                let id = ospf_rust_core::symbol::next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(id, &format!("recv_{}_{}", d, c), poly.monomials().to_vec(), *poly.constant_term())
            },
        );
        model.add_symbol_combination(&receive)?;

        // 6. 构建需求替换符号 (center x car_model)
        //
        // Kotlin equivalent: demand[d,c] = demands[c] - replacedDemand + replacedToDemand
        //   where replacedDemand = sum over replacements where from==c of demands[from]*y[r]
        //         replacedToDemand = sum over replacements where to==c of demands[from]*y[r]
        //
        // Rust formulation: demand symbol only contains replacement delta terms (no base demand).
        //   Constraint is: receive[d,c] + demand[d,c] >= demands[c]
        //   which is equivalent to: receive[d,c] >= demands[c] - demand[d,c]
        //                         = demands[c] - replacedDemand + replacedToDemand
        //   This matches Kotlin's: receive[d,c] >= demand[d,c] (with base demand in symbol).
        //
        // Sign convention:
        //   from==c: +demands[from]*y  (positive delta = reduces required shipment)
        //   to==c:   -demands[from]*y  (negative delta = increases required shipment)
        let y_idx_ref = &y_idx;
        let demand: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>> = SymbolCombination::new(
            Shape::new([centers.len(), car_models.len()]),
            "demand",
            |_idx, vec| {
                let d = vec[0];
                let c = vec[1];
                let mut terms = Vec::new();
                for (r_idx, replacement) in centers[d].replacements.iter().enumerate() {
                    if replacement.from == c {
                        // replacedDemand: model c's demand is partially fulfilled by another model
                        terms.push(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            centers[d].demands[replacement.from],
                            y_idx_ref[d][r_idx],
                        ));
                    }
                    if replacement.to == c {
                        // replacedToDemand: model c absorbs demand from another model
                        terms.push(ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            -centers[d].demands[replacement.from],
                            y_idx_ref[d][r_idx],
                        ));
                    }
                }
                let poly = ospf_rust_core::symbol::flatten::Linear::new(terms, 0.0);
                let id = ospf_rust_core::symbol::next_auto_intermediate_symbol_id();
                LinearExpressionSymbol::new(id, &format!("demand_{}_{}", d, c), poly.monomials().to_vec(), *poly.constant_term())
            },
        );
        model.add_symbol_combination(&demand)?;

        Ok(DistributionModel {
            x_vars,
            x_idx,
            y_vars,
            y_idx,
            cost,
            trans,
            receive,
            demand,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        manufacturers: &[Manufacturer],
        centers: &[Center],
        car_models: &[CarModel],
    ) -> Result<(), Box<dyn Error>> {
        // 7. 目标: 最小化成本
        let cost_poly = self.cost.symbol_polynomial(0);
        let cost_coeffs: Vec<_> = cost_poly.monomials().iter().map(|m| (m.var_index(), *m.coefficient())).collect();
        model.add_linear_objective(&cost_coeffs, "cost");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // 8. 需求约束: receive[d][c] + demand[d][c] >= centers[d].demands[c]
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                let recv_poly = self.receive.symbol_polynomial_at(&[d, c]);
                let dem_poly = self.demand.symbol_polynomial_at(&[d, c]);
                let mut coeffs: Vec<(usize, f64)> = recv_poly.monomials().iter()
                    .map(|m| (m.var_index(), *m.coefficient())).collect();
                for m in dem_poly.monomials() {
                    coeffs.push((m.var_index(), *m.coefficient()));
                }
                model.add_linear_constraint(&coeffs, ConstraintRelation::GreaterEqual, centers[d].demands[c], &format!("demand_{}_{}", d, c))?;
            }
        }

        // 9. 产能约束: trans[m][c] <= productivity
        for m in 0..manufacturers.len() {
            for c in 0..car_models.len() {
                if let Some(cap) = manufacturers[m].productivity_by_model[c] {
                    let poly = self.trans.symbol_polynomial_at(&[m, c]);
                    let coeffs: Vec<_> = poly.monomials().iter().map(|m| (m.var_index(), *m.coefficient())).collect();
                    model.add_linear_constraint(&coeffs, ConstraintRelation::LessEqual, cap, &format!("capacity_{}_{}", m, c))?;
                }
            }
        }

        Ok(())
    }
}

/// Demo15 主函数：多车型配送问题 / Demo15 main function: Multi-model vehicle distribution problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let car_models = build_car_models();
    let centers = build_centers();
    let manufacturers = build_manufacturers();

    let mut model = MetaModel::<f64>::new("demo15");
    let distribution = DistributionModel::register(&mut model, &manufacturers, &centers, &car_models)?;

    distribution.add_constraints(&mut model, &manufacturers, &centers, &car_models)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo15 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                let value = read_solution_value(&solution, distribution.x_idx[&[m, d, c]]);
                if value > 0.0 {
                    println!(
                        "{} -> {} {} = {:.2}",
                        manufacturers[m].name, centers[d].name, car_models[c].name, value
                    );
                }
            }
        }
    }
    for d in 0..centers.len() {
        for (r, replacement) in centers[d].replacements.iter().enumerate() {
            let ratio = read_solution_value(&solution, distribution.y_idx[d][r]);
            if ratio > 0.0 {
                println!(
                    "{} replace {} -> {} ratio {:.4}",
                    centers[d].name,
                    car_models[replacement.from].name,
                    car_models[replacement.to].name,
                    ratio
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo15() {
        assert!(run().is_ok());
    }
}
