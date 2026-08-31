use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1,
};
use ospf_rust_core::variable::{UInteger, VariableCombination2D};

use super::common::{read_solution_value, solve_typed};

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

/// Helper: extract (var_index, coefficient) pairs from a symbol
fn extract_coeffs(sym: &LinearExpressionSymbol<f64>) -> Vec<(usize, f64)> {
    let poly = sym.to_linear_polynomial();
    poly.monomials().iter()
        .map(|m| (m.var_index(), *m.coefficient()))
        .collect()
}

/// Demo7 主函数：运输问题 / Demo7 main function: Transportation problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let warehouses = build_warehouses();
    let stores = build_stores();

    let mut model = MetaModel::<f64>::new("demo7");
    let x_shape = Shape::new([warehouses.len(), stores.len()]);
    let x_vars: VariableCombination2D<UInteger> =
        VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
            format!("{}_{}", vector[0], vector[1])
        });
    let x_idx = model.register_combination(&x_vars)?;

    // 成本符号 / Cost symbol
    let cost = flat_map1("cost", &warehouses, |warehouse| {
        let w = warehouses.iter().position(|ww| ww.name == warehouse.name).unwrap();
        let monomials: Vec<_> = stores.iter().enumerate()
            .map(|(s, _)| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                warehouse.cost_to(s), x_idx[&[w, s]],
            ))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, warehouse| warehouse.name.clone());
    model.add_symbol_combination(&cost)?;

    // 每仓库发货符号 / Per-warehouse shipment symbol
    let shipment = flat_map1("shipment", &warehouses, |warehouse| {
        let w = warehouses.iter().position(|ww| ww.name == warehouse.name).unwrap();
        let monomials: Vec<_> = stores.iter().enumerate()
            .map(|(s, _)| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, warehouse| warehouse.name.clone());
    model.add_symbol_combination(&shipment)?;

    // 每商店采购符号 / Per-store purchase symbol
    let purchase = flat_map1("purchase", &stores, |store| {
        let s = stores.iter().position(|ss| ss.name == store.name).unwrap();
        let monomials: Vec<_> = warehouses.iter().enumerate()
            .map(|(w, _)| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[w, s]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, store| store.name.clone());
    model.add_symbol_combination(&purchase)?;

    // 目标: 最小化成本 / Objective: minimize cost
    let mut cost_coeffs = Vec::new();
    for w in 0..warehouses.len() {
        let poly = cost[w].to_linear_polynomial();
        for m in poly.monomials() {
            cost_coeffs.push((m.var_index(), *m.coefficient()));
        }
    }
    model.add_linear_objective(&cost_coeffs, "cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    // 仓库容量约束 / Warehouse capacity constraints
    for (w, warehouse) in warehouses.iter().enumerate() {
        let coeffs = extract_coeffs(&shipment[w]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::LessEqual,
            warehouse.stowage,
            &format!("stowage_{}", w),
        )?;
    }

    // 商店需求约束 / Store demand constraints
    for (s, store) in stores.iter().enumerate() {
        let coeffs = extract_coeffs(&purchase[s]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::GreaterEqual,
            store.demand,
            &format!("demand_{}", s),
        )?;
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo7 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, store) in stores.iter().enumerate() {
            let value = read_solution_value(&solution, x_idx[&[w, s]]);
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
