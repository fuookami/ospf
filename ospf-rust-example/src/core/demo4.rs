use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    SymbolCombination, LinearExpressionSymbol, flat_map1,
};
use ospf_rust_core::variable::{UContinuous, VariableCombination1D, VariableRange};

use super::common::{read_solution_value, solve_typed};

/// 材料数据结构 / Material data structure
#[derive(Debug, Clone)]
struct Material {
    name: String,
    available: f64,
}

impl Material {
    fn new(name: &str, available: f64) -> Self {
        Self { name: name.to_string(), available }
    }
}

/// 产品数据结构 / Product data structure
#[derive(Debug, Clone)]
struct Product {
    name: String,
    max_yield: f64,
    profit: f64,
    usage_by_material: Vec<f64>,
}

impl Product {
    fn new(name: &str, max_yield: f64, profit: f64, usage_by_material: Vec<f64>) -> Self {
        Self { name: name.to_string(), max_yield, profit, usage_by_material }
    }
}

fn build_materials() -> Vec<Material> {
    vec![Material::new("M0", 24.0), Material::new("M1", 8.0)]
}

fn build_products() -> Vec<Product> {
    vec![
        Product::new("P0", 3.0, 5.0, vec![6.0, 1.0]),
        Product::new("P1", 2.0, 4.0, vec![4.0, 2.0]),
    ]
}

/// 生产计划问题模型 / Production planning model
struct ProductionModel {
    x: VariableCombination1D<UContinuous>,
    x_idx: MultiArray<usize, Shape<1>>,
    profit: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    usage: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl ProductionModel {
    fn register(
        model: &mut MetaModel<f64>,
        products: &[Product],
        materials: &[Material],
    ) -> Result<Self, Box<dyn Error>> {
        // 1. 注册变量组合（带范围）
        let x = VariableCombination1D::with_range_generator(
            Shape::new([products.len()]),
            "x",
            |i, _| VariableRange::bounded(0.0, products[i].max_yield),
        );
        let x_idx = model.register_combination(&x)?;

        // 2. 利润符号
        let profit = flat_map1("profit", products, |p| {
            let i = products.iter().position(|pp| pp.name == p.name).unwrap();
            ospf_rust_core::symbol::flatten::Linear::new(
                vec![ospf_rust_core::symbol::flatten::LinearMonomial::new(p.profit, x_idx[i])],
                0.0,
            )
        }, |_, p| p.name.clone());
        model.add_symbol_combination(&profit)?;

        // 3. 材料用量符号
        let usage = flat_map1("usage", materials, |mat| {
            let m = materials.iter().position(|mm| mm.name == mat.name).unwrap();
            let monomials: Vec<_> = products.iter().enumerate().map(|(p_idx, p)| {
                ospf_rust_core::symbol::flatten::LinearMonomial::new(p.usage_by_material[m], x_idx[p_idx])
            }).collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, m| m.name.clone());
        model.add_symbol_combination(&usage)?;

        Ok(ProductionModel { x, x_idx, profit, usage })
    }

    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        products: &[Product],
        materials: &[Material],
    ) -> Result<(), Box<dyn Error>> {
        // 目标: 最大化利润
        let profit_poly = self.profit[0].to_linear_polynomial();
        let profit_coeffs: Vec<_> = profit_poly.monomials().iter()
            .map(|m| (m.var_index(), *m.coefficient())).collect();
        model.add_linear_objective(&profit_coeffs, "profit");
        model.set_objective_category(ObjectiveCategory::Maximum);

        // 材料约束
        for (m, mat) in materials.iter().enumerate() {
            let poly = self.usage[m].to_linear_polynomial();
            let coeffs: Vec<_> = poly.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(&coeffs, ConstraintRelation::LessEqual, mat.available, &format!("material_{}_{}", m, mat.name))?;
        }

        // 产品差异约束: x[p1] - x[p2] <= 1.0
        for p1 in 0..products.len() {
            for p2 in 0..products.len() {
                if p1 == p2 { continue; }
                let coefficients = vec![
                    (self.x_idx[p1], 1.0),
                    (self.x_idx[p2], -1.0),
                ];
                model.add_linear_constraint(&coefficients, ConstraintRelation::LessEqual, 1.0, &format!("diff_{}_{}", p1, p2))?;
            }
        }
        Ok(())
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let materials = build_materials();
    let products = build_products();

    let mut model = MetaModel::<f64>::new("demo4");
    let prod = ProductionModel::register(&mut model, &products, &materials)?;
    prod.add_constraints(&mut model, &products, &materials)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo4 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("profit: {:.2}", obj);
    }
    for (p, product) in products.iter().enumerate() {
        println!("{}: {:.4}", product.name, read_solution_value(&solution, prod.x_idx[p]));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_demo4() { assert!(run().is_ok()); }
}
