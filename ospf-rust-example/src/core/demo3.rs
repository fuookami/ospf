use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1_indexed,
};
use ospf_rust_core::variable::{UInteger, VariableCombination1D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// 材料数据结构 / Material data structure
#[derive(Debug, Clone)]
struct Material {
    name: String,
    unit_cost: f64,
    yields: Vec<f64>,
}

impl Material {
    fn new(name: &str, unit_cost: f64, yields: Vec<f64>) -> Self {
        Self { name: name.to_string(), unit_cost, yields }
    }
}

/// 产品目标数据结构 / Product target data structure
#[derive(Debug, Clone)]
struct ProductTarget {
    name: String,
    min_yield: f64,
}

impl ProductTarget {
    fn new(name: &str, min_yield: f64) -> Self {
        Self { name: name.to_string(), min_yield }
    }
}

fn build_materials() -> Vec<Material> {
    vec![
        Material::new("M0", 115.0, vec![30.0, 10.0, 0.0]),
        Material::new("M1", 97.0, vec![15.0, 0.0, 20.0]),
        Material::new("M2", 82.0, vec![0.0, 25.0, 15.0]),
        Material::new("M3", 76.0, vec![15.0, 15.0, 15.0]),
    ]
}

fn build_product_targets() -> Vec<ProductTarget> {
    vec![
        ProductTarget::new("P0", 15000.0),
        ProductTarget::new("P1", 15000.0),
        ProductTarget::new("P2", 10000.0),
    ]
}

/// 配料问题模型 / Blending problem model
struct BlendingModel {
    x: VariableCombination1D<UInteger>,
    x_idx: MultiArray<usize, Shape<1>>,
    cost: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    yields: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl BlendingModel {
    fn register(
        model: &mut MetaModel<f64>,
        materials: &[Material],
        targets: &[ProductTarget],
    ) -> Result<Self, Box<dyn Error>> {
        // 1. 注册变量组合
        let x = VariableCombination1D::new(Shape::new([materials.len()]), "x");
        let x_idx = model.register_combination(&x)?;

        // 2. 成本符号
        let cost = flat_map1_indexed("cost", materials, |m_idx, m| {
            let var_index = x_idx[m_idx];
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(m.unit_cost, var_index)],
                0.0,
            )
        }, |_, m| m.name.clone());
        model.add_symbol_combination(&cost)?;

        // 3. 产量符号
        let yields = flat_map1_indexed("yield", targets, |p, _t| {
            let monomials: Vec<_> = materials.iter().enumerate().filter_map(|(m_idx, m)| {
                let coeff = m.yields[p];
                if coeff != 0.0 {
                    Some(ospf_rust_core::symbol::flatten::LinearMonomial::new(coeff, x_idx[m_idx]))
                } else {
                    None
                }
            }).collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, t| t.name.clone());
        model.add_symbol_combination(&yields)?;

        Ok(BlendingModel { x, x_idx, cost, yields })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        targets: &[ProductTarget],
    ) -> Result<(), Box<dyn Error>> {
        // 目标: 最小化成本
        let cost_coeffs = extract_coeffs(&self.cost[0]);
        model.add_linear_objective(&cost_coeffs, "cost");
        model.set_objective_category(ObjectiveCategory::Minimum);

        // 产量约束
        for (p, target) in targets.iter().enumerate() {
            let coeffs = extract_coeffs(&self.yields[p]);
            model.add_linear_constraint(&coeffs, ConstraintRelation::GreaterEqual, target.min_yield, &format!("yield_{}_lb", target.name))?;
            model.add_linear_constraint(&coeffs, ConstraintRelation::LessEqual, target.min_yield, &format!("yield_{}_ub", target.name))?;
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let materials = build_materials();
    let targets = build_product_targets();

    let mut model = MetaModel::<f64>::new("demo3");
    let blending = BlendingModel::register(&mut model, &materials, &targets)?;
    blending.add_constraints(&mut model, &targets)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo3 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("total cost: {:.2}", obj);
    }
    for (m, _) in materials.iter().enumerate() {
        println!("{}: {:.2}", materials[m].name, read_solution_value(&solution, blending.x_idx[m]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_demo3() { assert!(run().is_ok()); }
}
