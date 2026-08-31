//! Demo2 模块 / Demo2 module
use std::error::Error;

use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::{LinearExpressionSymbol, flat_map1_indexed};
use ospf_rust_core::variable::{Binary, VariableCombination2D};
use ospf_rust_multiarray::Shape;

use super::common::{extract_coeffs, read_solution_value, solve_typed};

/// 产品数据结构 / Product data structure
#[derive(Debug, Clone)]
struct Product {
    /// 产品名称 / Product name
    name: String,
}

impl Product {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// 公司数据结构 / Company data structure
#[derive(Debug, Clone)]
struct Company {
    /// 公司名称 / Company name
    name: String,
    /// 各产品成本 / Costs for each product
    costs: Vec<f64>,
}

impl Company {
    fn new(name: &str, costs: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            costs,
        }
    }

    /// 获取指定产品的成本 / Get cost for specified product
    fn cost_of(&self, product_idx: usize) -> f64 {
        self.costs[product_idx]
    }
}

/// 构建产品列表 / Build product list
fn build_products() -> Vec<Product> {
    vec![
        Product::new("P0"),
        Product::new("P1"),
        Product::new("P2"),
        Product::new("P3"),
    ]
}

/// 构建公司列表 / Build company list
fn build_companies() -> Vec<Company> {
    vec![
        Company::new("C0", vec![920.0, 480.0, 650.0, 340.0]),
        Company::new("C1", vec![870.0, 510.0, 700.0, 350.0]),
        Company::new("C2", vec![880.0, 500.0, 720.0, 400.0]),
        Company::new("C3", vec![930.0, 490.0, 680.0, 410.0]),
    ]
}

/// 任务分配模型 / Assignment model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct TransportModel {
    /// 决策变量 / Decision variables
    x_vars: VariableCombination2D<Binary>,
    /// 模型索引数组 / Model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// 成本符号 / Cost symbol
    cost: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 每公司分配符号 / Per-company assignment symbol
    assignment_company:
        ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 每产品分配符号 / Per-product assignment symbol
    assignment_product:
        ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl TransportModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        companies: &[Company],
        products: &[Product],
    ) -> Result<Self, Box<dyn Error>> {
        let x_shape = Shape::new([companies.len(), products.len()]);
        let x_vars: VariableCombination2D<Binary> =
            VariableCombination2D::with_name_generator(x_shape.clone(), "x", |_index, vector| {
                format!("{}_{}", vector[0], vector[1])
            });
        let x_idx = model.register_combination(&x_vars)?;

        // 成本符号 / Cost symbol
        let cost = flat_map1_indexed(
            "cost",
            companies,
            |c, company| {
                let monomials: Vec<_> = products
                    .iter()
                    .enumerate()
                    .map(|(p, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            company.cost_of(p),
                            x_idx[&[c, p]],
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, company| company.name.clone(),
        );
        model.add_symbol_combination(&cost)?;

        // 每公司分配符号 / Per-company assignment symbol
        let assignment_company = flat_map1_indexed(
            "assign_company",
            companies,
            |c, _company| {
                let monomials: Vec<_> = products
                    .iter()
                    .enumerate()
                    .map(|(p, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[c, p]])
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, company| company.name.clone(),
        );
        model.add_symbol_combination(&assignment_company)?;

        // 每产品分配符号 / Per-product assignment symbol
        let assignment_product = flat_map1_indexed(
            "assign_product",
            products,
            |p, _product| {
                let monomials: Vec<_> = companies
                    .iter()
                    .enumerate()
                    .map(|(c, _)| {
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[c, p]])
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, product| product.name.clone(),
        );
        model.add_symbol_combination(&assignment_product)?;

        Ok(TransportModel {
            x_vars,
            x_idx,
            cost,
            assignment_company,
            assignment_product,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        companies: &[Company],
        products: &[Product],
    ) -> Result<(), Box<dyn Error>> {
        // 目标: 最小化成本 / Objective: minimize cost
        let mut cost_coeffs = Vec::new();
        for c in 0..companies.len() {
            let poly = self.cost.symbol_polynomial(c);
            for m in poly.monomials() {
                cost_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        let cost_input = LinearObjectiveInput::minimize("cost").terms(cost_coeffs.into_iter());
        model.set_linear_objective_input(cost_input);

        // 每公司最多分配1个产品 / Each company assigned at most 1 product
        for c in 0..companies.len() {
            let coeffs = extract_coeffs(&self.assignment_company[c]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("company_{}", c),
            )?;
        }

        // 每产品恰好分配1个公司 / Each product assigned exactly 1 company
        for p in 0..products.len() {
            let coeffs = extract_coeffs(&self.assignment_product[p]);
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::Equal,
                1.0,
                &format!("product_{}", p),
            )?;
        }

        Ok(())
    }
}

/// Demo2 主函数：任务分配问题 / Demo2 main function: Assignment problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let companies = build_companies();
    let products = build_products();

    let mut model = MetaModel::<f64>::new("demo2");
    let transport = TransportModel::register(&mut model, &companies, &products)?;

    transport.add_constraints(&mut model, &companies, &products)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo2 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("total cost: {:.2}", obj);
    }
    for (c, company) in companies.iter().enumerate() {
        for (p, product) in products.iter().enumerate() {
            if read_solution_value(&solution, transport.x_idx[&[c, p]]) > 0.5 {
                println!("assign {} -> {}", product.name, company.name);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo2() {
        assert!(run().is_ok());
    }
}
