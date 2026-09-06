//! Demo5 模块 / Demo5 module
use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::{LinearExpressionSymbol, SymbolCombination, flat_map1_indexed};
use ospf_rust_core::variable::{Binary, VariableCombination1D};
use ospf_rust_multiarray::{MultiArray, Shape};

use super::common::{extract_coeffs, read_solution_value, solve_typed};

/// 货物数据结构 / Cargo data structure
#[derive(Debug, Clone)]
struct Cargo {
    name: String,
    weight: f64,
    value: f64,
}

impl Cargo {
    fn new(name: &str, weight: f64, value: f64) -> Self {
        Self {
            name: name.to_string(),
            weight,
            value,
        }
    }
}

fn build_cargos() -> Vec<Cargo> {
    vec![
        Cargo::new("C0", 2.0, 6.0),
        Cargo::new("C1", 2.0, 3.0),
        Cargo::new("C2", 6.0, 5.0),
        Cargo::new("C3", 5.0, 4.0),
        Cargo::new("C4", 4.0, 6.0),
    ]
}

/// 0-1 背包问题模型 / 0-1 Knapsack model
struct KnapsackModel {
    x: VariableCombination1D<Binary>,
    x_idx: MultiArray<usize, Shape<1>>,
    cargo_value: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    cargo_weight: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl KnapsackModel {
    fn register(model: &mut MetaModel<f64>, cargos: &[Cargo]) -> Result<Self, Box<dyn Error>> {
        let x = VariableCombination1D::new(Shape::new([cargos.len()]), "x");
        let x_idx = model.register_combination(&x)?;

        let cargo_value = flat_map1_indexed(
            "cargo_value",
            cargos,
            |i, c| {
                ospf_rust_core::symbol::flatten::Linear::new(
                    vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                        c.value, x_idx[i],
                    )],
                    0.0,
                )
            },
            |_, c| c.name.clone(),
        );
        model.add_symbol_combination(&cargo_value)?;

        let cargo_weight = flat_map1_indexed(
            "cargo_weight",
            cargos,
            |i, c| {
                ospf_rust_core::symbol::flatten::Linear::new(
                    vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(
                        c.weight, x_idx[i],
                    )],
                    0.0,
                )
            },
            |_, c| c.name.clone(),
        );
        model.add_symbol_combination(&cargo_weight)?;

        Ok(KnapsackModel {
            x,
            x_idx,
            cargo_value,
            cargo_weight,
        })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        max_weight: f64,
    ) -> Result<(), Box<dyn Error>> {
        // 目标: 最大化总价值
        let val_coeffs = extract_coeffs(&self.cargo_value[0]);
        model.add_linear_objective(&val_coeffs, "value");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // 约束: 总重量 <= max_weight
        let wt_coeffs = extract_coeffs(&self.cargo_weight[0]);
        model.add_linear_constraint(
            &wt_coeffs,
            ConstraintRelation::LessEqual,
            max_weight,
            "weight",
        )?;

        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let cargos = build_cargos();
    let max_weight = 10.0;

    let mut model = MetaModel::<f64>::new("demo5");
    let knapsack = KnapsackModel::register(&mut model, &cargos)?;
    knapsack.add_constraints(&mut model, max_weight)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo5 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("value: {:.2}", obj);
    }
    for (i, cargo) in cargos.iter().enumerate() {
        if read_solution_value(&solution, knapsack.x_idx[i]) > 0.5 {
            println!("pick {}", cargo.name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_demo5() {
        assert!(run().is_ok());
    }
}
