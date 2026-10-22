//! 启发式求解示例 / Heuristic solving example
//!
//! 演示「配置终止条件 → 选择算法入口 → 求解模型 → 读取结果」的完整流程。
//! Demonstrates the full flow "configure stopping → pick an algorithm entry → solve → read the result".

use std::sync::Arc;

use ospf_rust_core::model::callback::{AbstractCallBackModel, Solution};
use ospf_rust_core::solver::heuristic::composition::SearchConfig;
use ospf_rust_core::solver::heuristic::facade::hill_climbing;
use ospf_rust_core::solver::heuristic::{HeuristicSolver, Iteration, SeededEntropy};
use ospf_rust_core::variable::new_standalone_id;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 一维二次问题：min (x - 1)^2，x ∈ [-10, 10]。
    // 1D quadratic: min (x - 1)^2 with x ∈ [-10, 10].
    let x = new_standalone_id();
    let mut model = AbstractCallBackModel::<f64>::new("heuristic_demo");
    model.register_variable_name(x, "x".to_string());
    model.set_variable_bounds_provider(Arc::new(move |id| {
        if id == x {
            Some((Some(-10.0), Some(10.0)))
        } else {
            None
        }
    }));
    model.set_initial_solution_provider(Arc::new(move |amount| {
        (0..amount)
            .map(|index| Solution::<f64>::from([(x, 5.0 + index as f64)]))
            .collect()
    }));
    model.set_objective_evaluator(Arc::new(|solution| {
        let value = solution.values().next().copied().unwrap_or(0.0);
        Some((value - 1.0) * (value - 1.0))
    }));

    let config = SearchConfig::new(200).with_not_better_limit(50);
    let mut solver = hill_climbing(config.stopping(), Box::new(SeededEntropy::new(42)));

    let result = solver.solve(&mut model, &mut Iteration::new())?;
    println!("algorithm = {}", solver.name());
    println!("status = {:?}", result.status);
    if let (Some(objective), Some(solution)) = (result.best_objective, result.best_solution) {
        println!("best objective = {objective}");
        println!("x = {:?}", solution.get(&x));
    }

    Ok(())
}
