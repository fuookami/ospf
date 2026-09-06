//! Demo7 模块 / Demo7 module
use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::{LinearExpressionSymbol, flat_map1_indexed};
use ospf_rust_core::variable::{UInteger, VariableCombination2D};
use ospf_rust_multiarray::Shape;

use super::common::{extract_coeffs, read_solution_value, solve_typed};

/// 仓库数据结构 / Warehouse data structure
#[derive(Debug, Clone)]
struct Warehouse {
    /// 仓库名称 / Warehouse name
    name: String,
    /// 存储量 / Storage capacity
    stowage: f64,
    /// 到各商店的运输成本 / Transportation costs to each store
    costs_to_stores: Vec<f64>,
}

impl Warehouse {
    fn new(name: &str, stowage: f64, costs_to_stores: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            stowage,
            costs_to_stores,
        }
    }

    /// 获取到指定商店的运输成本 / Get cost to specified store
    fn cost_to(&self, store_idx: usize) -> f64 {
        self.costs_to_stores[store_idx]
    }
}

/// 商店数据结构 / Store data structure
#[derive(Debug, Clone)]
struct Store {
    /// 商店名称 / Store name
    name: String,
    /// 需求量 / Demand
    demand: f64,
}

impl Store {
    fn new(name: &str, demand: f64) -> Self {
        Self {
            name: name.to_string(),
            demand,
        }
    }
}

/// 构建仓库列表 / Build warehouse list
fn build_warehouses() -> Vec<Warehouse> {
    vec![
        Warehouse::new("W0", 510.0, vec![12.0, 13.0, 21.0, 7.0]),
        Warehouse::new("W1", 470.0, vec![14.0, 17.0, 8.0, 18.0]),
        Warehouse::new("W2", 520.0, vec![10.0, 11.0, 9.0, 15.0]),
    ]
}

/// 构建商店列表 / Build store list
fn build_stores() -> Vec<Store> {
    vec![
        Store::new("S0", 200.0),
        Store::new("S1", 400.0),
        Store::new("S2", 600.0),
        Store::new("S3", 300.0),
    ]
}

/// 运输问题模型 / Transportation problem model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct InventoryModel {
    /// 决策变量 / Decision variables
    x_vars: VariableCombination2D<UInteger>,
    /// 模型索引数组 / Model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 成本符号 / Cost symbol
    cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 每仓库发货符号 / Per-warehouse shipment symbol
    shipment: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 每商店采购符号 / Per-store purchase symbol
    purchase: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl InventoryModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        warehouses: &[Warehouse],
        stores: &[Store],
    ) -> Result<Self, Box<dyn Error>> {
        let x_shape = Shape::new([warehouses.len(), stores.len()]);
        let x_vars: VariableCombination2D<UInteger> =
            VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
                format!("{}_{}", vector[0], vector[1])
            });
        let x_idx = model.register_combination(&x_vars)?;

        // 成本符号 / Cost symbol
        let cost = flat_map1_indexed(
            "cost",
            warehouses,
            |w, warehouse| {
                let monomials: Vec<_> = stores
                    .iter()
                    .enumerate()
                    .map(|(s, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            warehouse.cost_to(s),
                            x_idx[&[w, s]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, warehouse| warehouse.name.clone(),
        );
        model.add_symbol_combination(&cost)?;

        // 每仓库发货符号 / Per-warehouse shipment symbol
        let shipment = flat_map1_indexed(
            "shipment",
            warehouses,
            |w, _warehouse| {
                let monomials: Vec<_> = stores
                    .iter()
                    .enumerate()
                    .map(|(s, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, warehouse| warehouse.name.clone(),
        );
        model.add_symbol_combination(&shipment)?;

        // 每商店采购符号 / Per-store purchase symbol
        let purchase = flat_map1_indexed(
            "purchase",
            stores,
            |s, _store| {
                let monomials: Vec<_> = warehouses
                    .iter()
                    .enumerate()
                    .map(|(w, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]])
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, store| store.name.clone(),
        );
        model.add_symbol_combination(&purchase)?;

        Ok(InventoryModel {
            x_vars,
            x_idx,
            cost,
            shipment,
            purchase,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        warehouses: &[Warehouse],
        stores: &[Store],
    ) -> Result<(), Box<dyn Error>> {
        // 目标: 最小化成本 / Objective: minimize cost
        let mut cost_coeffs = Vec::new();
        for w in 0..warehouses.len() {
            let poly = self.cost.symbol_polynomial(w);
            for m in poly.monomials() {
                cost_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        let cost_input = LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter());
        model.set_linear_objective_input(cost_input);

        // 仓库容量约束 / Warehouse capacity constraints
        for w in 0..warehouses.len() {
            let coeffs = extract_coeffs(&self.shipment[w]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                warehouses[w].stowage,
                &format!("stowage_{}", w),
            )?;
        }

        // 商店需求约束 / Store demand constraints
        for s in 0..stores.len() {
            let coeffs = extract_coeffs(&self.purchase[s]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::GreaterEqual,
                stores[s].demand,
                &format!("demand_{}", s),
            )?;
        }

        Ok(())
    }
}

/// Demo7 主函数：运输问题 / Demo7 main function: Transportation problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let warehouses = build_warehouses();
    let stores = build_stores();

    let mut model = MetaModel::<f64>::new("demo7");
    let inventory = InventoryModel::register(&mut model, &warehouses, &stores)?;

    inventory.add_constraints(&mut model, &warehouses, &stores)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo7 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, store) in stores.iter().enumerate() {
            let value = read_solution_value(&solution, inventory.x_idx[&[w, s]]);
            if value >= 1.0 {
                println!("{} -> {} = {:.2}", warehouse.name, store.name, value);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo7() {
        assert!(run().is_ok());
    }
}
