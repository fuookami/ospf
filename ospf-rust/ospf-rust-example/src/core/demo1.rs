//! Demo1 - 投资组合选择问题
//! Demo1 - Portfolio Selection Problem
//!
//! 问题描述 / Problem Description:
//! 有5家公司，每家都有资本、负债和利润三个属性。
//! There are 5 companies, each with capital, liability, and profit attributes.
//!
//! 目标：选择一组公司使得总利润最大化
//! Objective: Select a set of companies to maximize total profit
//!
//! 约束：
//! Constraints:
//! - 总资本 >= 10.0 / Total capital >= 10.0
//! - 总负债 <= 5.0 / Total liability <= 5.0
//!
//! 本示例展示 core 层 SymbolCombination 新 API 的完整建模链路：
//! This example demonstrates the complete modeling workflow with SymbolCombination:
//! 1. `register_combination` 批量注册变量组合
//! 2. `flat_map1` 从变量组合派生符号组合
//! 3. `add_symbol_combination` 批量注册中间符号
//! 4. 约束和目标从显式 `LinearExpressionSymbol` 提取多项式

use super::common::{extract_coeffs, solve_typed};
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::symbol::{LinearExpressionSymbol, SymbolCombination, flat_map1};
use ospf_rust_core::variable::{Binary, VariableCombination1D};
use ospf_rust_multiarray::{MultiArray, Shape};

/// 公司数据结构 / Company data structure
#[derive(Debug, Clone)]
struct Company {
    name: String,
    capital: f64,
    liability: f64,
    profit: f64,
}

impl Company {
    fn new(name: &str, capital: f64, liability: f64, profit: f64) -> Self {
        Self {
            name: name.to_string(),
            capital,
            liability,
            profit,
        }
    }
}

/// 公司数据 / Company data
fn get_companies() -> Vec<Company> {
    vec![
        Company::new("Company A", 3.48, 1.28, 5400.0),
        Company::new("Company B", 5.62, 2.53, 2300.0),
        Company::new("Company C", 7.33, 1.02, 4600.0),
        Company::new("Company D", 6.27, 3.55, 3300.0),
        Company::new("Company E", 2.14, 0.53, 980.0),
    ]
}

/// 指标类型 / Metric type
#[derive(Debug, Clone, Copy)]
enum Metric {
    Capital,
    Liability,
    Profit,
}

impl Metric {
    fn name(&self) -> &str {
        match self {
            Metric::Capital => "total_capital",
            Metric::Liability => "total_liability",
            Metric::Profit => "total_profit",
        }
    }

    fn value(&self, company: &Company) -> f64 {
        match self {
            Metric::Capital => company.capital,
            Metric::Liability => company.liability,
            Metric::Profit => company.profit,
        }
    }
}

/// 投资组合选择模型 / Portfolio selection model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct PortfolioModel {
    /// 决策变量：是否选择该公司
    select: VariableCombination1D<Binary>,
    /// 模型索引数组
    select_idx: MultiArray<usize, Shape<1>>,
    /// 派生指标符号组合：capital, liability, profit
    metrics: SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
}

impl PortfolioModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        companies: &[Company],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. 创建并注册变量组合
        let select = VariableCombination1D::new(Shape::new([companies.len()]), "select");
        let select_idx = model.register_combination(&select)?;

        // 2. 从变量组合派生符号组合
        //    metrics[0] = total_capital, metrics[1] = total_liability, metrics[2] = total_profit
        let metrics_list = vec![Metric::Capital, Metric::Liability, Metric::Profit];
        let select_ref = &select;
        let select_idx_ref = &select_idx;
        let metrics = flat_map1(
            "portfolio_metric",
            &metrics_list,
            |metric| {
                // 构建 Linear<f64>：sum(metric_value[i] * select[i])
                let monomials: Vec<_> = companies
                    .iter()
                    .enumerate()
                    .map(|(i, company)| {
                        let var_index = select_idx_ref[i];
                        ospf_rust_core::symbol::flatten::LinearMonomial::new(
                            metric.value(company),
                            var_index,
                        )
                    })
                    .collect();
                ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
            },
            |_, metric| metric.name().to_string(),
        );
        model.add_symbol_combination(&metrics)?;

        Ok(PortfolioModel {
            select,
            select_idx,
            metrics,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        min_capital: f64,
        max_liability: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 约束1: 总资本 >= min_capital
        let cap_coeffs = extract_coeffs(&self.metrics[0]);
        model.add_linear_constraint(
            &cap_coeffs,
            ospf_rust_core::model::ConstraintRelation::GreaterEqual,
            min_capital,
            "capital_constraint",
        )?;

        // 约束2: 总负债 <= max_liability
        let lia_coeffs = extract_coeffs(&self.metrics[1]);
        model.add_linear_constraint(
            &lia_coeffs,
            ospf_rust_core::model::ConstraintRelation::LessEqual,
            max_liability,
            "liability_constraint",
        )?;

        // 目标: 最大化总利润
        let obj_coeffs = extract_coeffs(&self.metrics[2]);
        model.add_linear_objective(&obj_coeffs, "total_profit");
        model.set_objective_category(ObjectiveCategory::Maximum);

        Ok(())
    }
}

/// Demo1 主函数 / Demo1 main function
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Demo1: Portfolio Selection Problem ===\n");

    let companies = get_companies();
    let min_capital = 10.0;
    let max_liability = 5.0;

    // 创建元模型 / Create meta model
    let mut meta_model = MetaModel::new("portfolio_selection");

    // 注册模型：变量组合 + 符号组合
    let portfolio = PortfolioModel::register(&mut meta_model, &companies)?;

    println!(
        "Step 1: Registered variable combination: select ({} vars)",
        portfolio.select.len()
    );
    println!(
        "Step 2: Registered symbol combination: metrics ({} symbols)",
        portfolio.metrics.len()
    );
    for i in 0..portfolio.metrics.len() {
        println!(
            "  metrics[{}] = {} (ID: {})",
            i,
            portfolio.metrics.symbol_name(i),
            portfolio.metrics.symbol_id(i)
        );
    }
    println!();

    // 添加约束和目标
    portfolio.add_constraints(&mut meta_model, min_capital, max_liability)?;
    println!("Step 3: Added constraints and objective from symbol polynomials");
    println!("  capital >= {}", min_capital);
    println!("  liability <= {}", max_liability);
    println!("  maximize total_profit");
    println!();

    // 模型转换
    println!("Step 4: Model transformation chain");
    let mechanism_model = meta_model.try_to_mechanism_model()?;
    println!(
        "  MechanismModel: {} variables, {} constraints",
        mechanism_model.num_variables(),
        mechanism_model.num_constraints()
    );

    let linear_model = mechanism_model.into_linear_triad_model();
    println!(
        "  LinearTriadModel: {} variables, {} constraints",
        linear_model.num_variables(),
        linear_model.num_constraints()
    );
    println!();

    // 求解
    println!("Step 5: Solving...");
    let output = solve_typed(meta_model)?;
    println!("Status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("Objective value (total profit): {:.0}", obj);
    }

    // 提取结果
    let solution = output.solution;
    println!("\n=== Solution ===");
    let mut selected: Vec<String> = Vec::new();
    let mut total_capital = 0.0;
    let mut total_liability = 0.0;

    for (i, var) in portfolio.select.iter().enumerate() {
        let val = solution.get(i).copied().unwrap_or(0.0);
        println!("  {} = {:.6}", var.name(), val);
        if val > 0.5 {
            selected.push(companies[i].name.clone());
            total_capital += companies[i].capital;
            total_liability += companies[i].liability;
        }
    }

    println!("\n=== Results ===");
    println!("Selected companies: {:?}", selected);
    println!(
        "Total capital: {:.2} (required >= {:.2})",
        total_capital, min_capital
    );
    println!(
        "Total liability: {:.2} (required <= {:.2})",
        total_liability, max_liability
    );

    // 验证
    println!("\n=== Constraint Verification ===");
    if total_capital >= min_capital {
        println!("  Capital constraint satisfied");
    } else {
        println!("  Capital constraint VIOLATED!");
    }
    if total_liability <= max_liability {
        println!("  Liability constraint satisfied");
    } else {
        println!("  Liability constraint VIOLATED!");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo1() {
        assert!(run().is_ok());
    }
}
