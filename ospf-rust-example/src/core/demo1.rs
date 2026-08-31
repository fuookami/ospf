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
//! 本示例展示使用 MetaModel 进行高层建模的完整流程：
//! This example demonstrates the complete workflow of high-level modeling using MetaModel:
//! 1. 在 MetaModel 中注册决策变量
//!    Register decision variables in MetaModel
//! 2. 使用运算符重载在 MetaModel 中添加约束条件
//!    Add constraints in MetaModel using operator overloading
//! 3. 在 MetaModel 中设置目标函数
//!    Set objective function in MetaModel
//! 4. 转换: MetaModel -> MechanismModel -> LinearTriadModel
//!    Transform: MetaModel -> MechanismModel -> LinearTriadModel
//! 5. 求解 / Solve

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::solver::{LinearSolver, solvers::GurobiSolver};
use ospf_rust_core::variable::{Binary, VariableCombination1D};
use ospf_rust_math::symbol::Linear;
use ospf_rust_multiarray::Shape;

/// 公司数据结构 / Company data structure
#[derive(Debug, Clone)]
struct Company {
    /// 名称 / Name
    name: String,
    /// 资本 / Capital
    capital: f64,
    /// 负债 / Liability
    liability: f64,
    /// 利润 / Profit
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

/// Demo1 主函数 / Demo1 main function
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Demo1: Portfolio Selection Problem ===\n");

    // ========================================================================
    // 步骤 1: 使用 MetaModel 进行高层建模
    // Step 1: High-level modeling using MetaModel
    // ========================================================================

    // 公司数据 / Company data
    let companies = get_companies();

    // 参数 / Parameters
    let min_capital = 10.0;
    let max_liability = 5.0;

    // 创建元模型 / Create meta model
    let mut meta_model = MetaModel::new("portfolio_selection");

    // 使用 VariableCombination 创建决策变量
    // Use VariableCombination to create decision variables
    let decision_vars: VariableCombination1D<Binary> =
        VariableCombination1D::new(Shape::new([companies.len()]), "select");

    // 注册变量到模型 / Register variables to model
    for var in decision_vars.iter() {
        meta_model.register_variable(var.clone())?;
    }

    println!("Step 1: Created {} decision variables", decision_vars.len());
    for (i, var) in decision_vars.iter().enumerate() {
        println!("  {} = x_{} (ID: {})", var.name(), i, var.id());
    }
    println!();

    // ========================================================================
    // 步骤 2: 使用运算符重载在 MetaModel 中添加约束
    // Step 2: Add constraints in MetaModel using operator overloading
    // ========================================================================

    // 收集变量引用用于构建约束表达式
    // Collect variable references for building constraint expressions
    let var_refs: Vec<_> = decision_vars.iter().collect();

    // 约束1: 总资本 >= min_capital
    // 使用运算符重载和 Linear::ge() 便捷方法
    // Using operator overloading and Linear::ge() convenience method
    let mut capital_expr = Linear::zero();
    for (i, company) in companies.iter().enumerate() {
        capital_expr = capital_expr + company.capital * var_refs[i];
    }
    meta_model.add_math_inequality(capital_expr.ge(min_capital), "capital_constraint");
    println!(
        "Step 2: Added constraint: total capital >= {} (using Linear::ge())",
        min_capital
    );

    // 约束2: 总负债 <= max_liability
    // 使用运算符重载和 Linear::le() 便捷方法
    // Using operator overloading and Linear::le() convenience method
    let mut liability_expr = Linear::zero();
    for (i, company) in companies.iter().enumerate() {
        liability_expr = liability_expr + company.liability * var_refs[i];
    }
    meta_model.add_math_inequality(liability_expr.le(max_liability), "liability_constraint");
    println!(
        "  Added constraint: total liability <= {} (using Linear::le())",
        max_liability
    );
    println!();

    // ========================================================================
    // 步骤 3: 在 MetaModel 中设置目标函数
    // Step 3: Set objective function in MetaModel
    // ========================================================================

    // 最大化总利润 / Maximize total profit
    let profit_coeffs: Vec<f64> = companies.iter().map(|c| c.profit).collect();
    meta_model.set_linear_objective(profit_coeffs.clone(), ObjectiveCategory::Maximum);

    println!("Step 3: Set objective: maximize total profit");
    println!("  Coefficients: {:?}", profit_coeffs);
    println!();

    // ========================================================================
    // 步骤 4: 模型转换 MetaModel -> MechanismModel -> LinearTriadModel
    // Step 4: Model transformation MetaModel -> MechanismModel -> LinearTriadModel
    // ========================================================================

    println!("Step 4: Model transformation chain");

    // 4.1: MetaModel -> MechanismModel
    // 符号约束在转换时自动映射到整数索引
    // Symbolic constraints are automatically mapped to integer indices during transformation
    let mechanism_model = meta_model.into_mechanism_model();
    println!("  4.1: MetaModel -> MechanismModel");
    println!("       Variables: {}", mechanism_model.num_variables());
    println!("       Constraints: {}", mechanism_model.num_constraints());

    // 4.2: MechanismModel -> LinearTriadModel
    let linear_model = mechanism_model.into_linear_triad_model();
    println!("  4.2: MechanismModel -> LinearTriadModel");
    println!("       Variables: {}", linear_model.num_variables());
    println!("       Constraints: {}", linear_model.num_constraints());
    println!();

    // 打印模型摘要 / Print model summary
    println!("=== Model Summary ===");
    println!("Model: {}", linear_model.name);
    println!("Variables: {} (binary)", linear_model.num_variables());
    println!("Constraints: {}", linear_model.num_constraints());
    println!("Objective: Maximize total profit");
    println!();

    // ========================================================================
    // 步骤 5: 求解 / Step 5: Solve
    // ========================================================================

    println!("Step 5: Solving with Gurobi...");
    let solver = GurobiSolver::new();

    match solver.solve_linear(&linear_model) {
        Ok(output) => {
            println!("\n=== Solver Output ===");
            println!("Status: {:?}", output.status);

            if let Some(obj) = output.objective_value {
                println!("Objective value (total profit): {:.0}", obj);
            }

            if let Some(ref solution) = output.solution {
                println!("\n=== Solution ===");
                let mut selected: Vec<String> = Vec::new();
                let mut total_capital = 0.0;
                let mut total_liability = 0.0;

                for (i, var) in decision_vars.iter().enumerate() {
                    let val = solution[i];
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

                // 验证约束 / Verify constraints
                println!("\n=== Constraint Verification ===");
                if total_capital >= min_capital {
                    println!("✓ Capital constraint satisfied");
                } else {
                    println!("✗ Capital constraint violated!");
                }
                if total_liability <= max_liability {
                    println!("✓ Liability constraint satisfied");
                } else {
                    println!("✗ Liability constraint violated!");
                }
            }

            if let Some(iter) = output.iterations {
                println!("\nIterations: {}", iter);
            }

            println!("Solve time: {:?}", output.solve_time);
        }
        Err(e) => {
            println!("Solver error: {}", e);
            return Err(e.into());
        }
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
