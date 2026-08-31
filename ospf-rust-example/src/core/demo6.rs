use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1_indexed,
};
use ospf_rust_core::variable::{UInteger, VariableCombination1D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

#[derive(Debug, Clone)]
struct Cargo {
    name: String,
    weight: f64,
    value: f64,
    max_amount: f64,
}

impl Cargo {
    fn new(name: &str, weight: f64, value: f64, max_amount: f64) -> Self {
        Self { name: name.to_string(), weight, value, max_amount }
    }
}

fn build_cargos() -> Vec<Cargo> {
    vec![
        Cargo::new("C0", 1.0, 6.0, 10.0),
        Cargo::new("C1", 2.0, 10.0, 5.0),
        Cargo::new("C2", 2.0, 20.0, 2.0),
    ]
}

struct IntegerKnapsackModel {
    x: VariableCombination1D<UInteger>,
    x_idx: MultiArray<usize, Shape<1>>,
    cargo_value: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    cargo_weight: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl IntegerKnapsackModel {
    fn register(model: &mut MetaModel<f64>, cargos: &[Cargo]) -> Result<Self, Box<dyn Error>> {
        let x = VariableCombination1D::new(Shape::new([cargos.len()]), "x");
        let x_idx = model.register_combination(&x)?;

        let cargo_value = flat_map1_indexed("cargo_value", cargos, |i, c| {
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(c.value, x_idx[i])], 0.0)
        }, |_, c| c.name.clone());
        model.add_symbol_combination(&cargo_value)?;

        let cargo_weight = flat_map1_indexed("cargo_weight", cargos, |i, c| {
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(c.weight, x_idx[i])], 0.0)
        }, |_, c| c.name.clone());
        model.add_symbol_combination(&cargo_weight)?;

        Ok(IntegerKnapsackModel { x, x_idx, cargo_value, cargo_weight })
    }

    fn add_constraints(&self, model: &mut MetaModel<f64>, cargos: &[Cargo], max_weight: f64) -> Result<(), Box<dyn Error>> {
        // 目标: 最大化价值
        let val_coeffs = extract_coeffs(&self.cargo_value[0]);
        model.add_linear_objective(&val_coeffs, "value");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // 重量约束
        let wt_coeffs = extract_coeffs(&self.cargo_weight[0]);
        model.add_linear_constraint(&wt_coeffs, ConstraintRelation::LessEqual, max_weight, "weight")?;

        // 上界约束
        for (i, cargo) in cargos.iter().enumerate() {
            model.add_linear_constraint(
                &[(self.x_idx[i], 1.0)],
                ConstraintRelation::LessEqual,
                cargo.max_amount,
                &format!("upper_{}", i),
            )?;
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let cargos = build_cargos();
    let max_weight = 8.0;

    let mut model = MetaModel::<f64>::new("demo6");
    let knapsack = IntegerKnapsackModel::register(&mut model, &cargos)?;
    knapsack.add_constraints(&mut model, &cargos, max_weight)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo6 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value { println!("value: {:.2}", obj); }
    for (i, cargo) in cargos.iter().enumerate() {
        println!("{}: {:.2}", cargo.name, read_solution_value(&solution, knapsack.x_idx[i]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_demo6() { assert!(run().is_ok()); }
}
