use std::error::Error;

use ospf_rust_multiarray::{MultiArray, Shape};
use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol, flat_map1};
use ospf_rust_core::variable::{Binary, VariableCombination2D};

use super::common::{
    add_constraint_with_metadata, linear_expr_from_indices, read_solution_value,
    register_binary_matrix, set_linear_objective_from_sparse_terms, solve_typed,
};

/// 工人数据结构 / Worker data structure
#[derive(Clone)]
struct Worker {
    /// 工人名称 / Worker name
    name: String,
}

impl Worker {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// 任务数据结构 / Task data structure
#[derive(Clone)]
struct Task {
    /// 任务名称 / Task name
    name: String,
}

impl Task {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// 分配数据结构 / Assignment data structure
#[derive(Clone)]
struct AssignmentData {
    /// 工人列表 / Worker list
    workers: Vec<Worker>,
    /// 任务列表 / Task list
    tasks: Vec<Task>,
    /// 成本矩阵 / Cost matrix
    costs: Vec<Vec<f64>>,
}

impl AssignmentData {
    fn sample() -> Self {
        Self {
            workers: vec![Worker::new("Alice"), Worker::new("Bob")],
            tasks: vec![Task::new("Packing"), Task::new("Delivery")],
            costs: vec![vec![4.0, 8.0], vec![6.0, 3.0]],
        }
    }
}

/// 构建低层模型（显式 sparse 风格，教学对比用）
/// Build low-level model (explicit sparse style, for teaching comparison)
///
/// 注意：此路径保留 `register_binary_matrix` 等低层 helper，
/// 仅作为 legacy/teaching 对比。推荐主路径使用 `build_shortcut_model`。
fn build_low_level_model(
    data: &AssignmentData,
) -> Result<(MetaModel<f64>, Vec<Vec<usize>>), Box<dyn Error>> {
    let mut model = MetaModel::<f64>::new("core_shortcuts_low_level");
    let idx = register_binary_matrix(&mut model, data.workers.len(), data.tasks.len(), "x")?;

    // 目标
    let mut objective_terms: Vec<(usize, f64)> =
        Vec::with_capacity(data.workers.len() * data.tasks.len());
    for w in 0..data.workers.len() {
        for t in 0..data.tasks.len() {
            objective_terms.push((idx[w][t], data.costs[w][t]));
        }
    }
    set_linear_objective_from_sparse_terms(model, &objective_terms, ObjectiveCategory::Minimum);

    // 工人容量约束
    for (w, worker) in data.workers.iter().enumerate() {
        let indices: Vec<usize> = (0..data.tasks.len()).map(|t| idx[w][t]).collect();
        let coefficients = linear_expr_from_indices(&indices, 1.0);
        add_constraint_with_metadata(
            model, &coefficients, ConstraintRelation::LessEqual, 1.0,
            &format!("worker_capacity_{}", worker.name), None, false, 0, None,
        )?;
    }

    // 任务分配约束
    for (t, task) in data.tasks.iter().enumerate() {
        let indices: Vec<usize> = (0..data.workers.len()).map(|w| idx[w][t]).collect();
        let coefficients = linear_expr_from_indices(&indices, 1.0);
        add_constraint_with_metadata(
            model, &coefficients, ConstraintRelation::Equal, 1.0,
            &format!("task_partition_{}", task.name), None, false, 0, None,
        )?;
    }

    // 偏好约束
    add_constraint_with_metadata(
        model, &[(idx[0][0], 1.0), (idx[1][1], 1.0)],
        ConstraintRelation::GreaterEqual, 1.0,
        "prefer_diagonal_low_level", None, false, 0, None,
    )?;

    Ok((model, idx))
}

/// 构建组合式字段化模型（推荐主路径）
/// Build combination-based field model (recommended main path)
///
/// 使用 `VariableCombination2D<Binary>` + `register_combination` + 符号组合派生。
fn build_shortcut_model(
    data: &AssignmentData,
) -> Result<(MetaModel<f64>, MultiArray<usize, Shape<2>>), Box<dyn Error>> {
    let mut model = MetaModel::<f64>::new("core_shortcuts_shortcut");

    // 1. 注册变量组合
    let x = VariableCombination2D::<Binary>::with_name_and_range_generator(
        Shape::new([data.workers.len(), data.tasks.len()]),
        "x",
        |_index, vector| format!("{}_{}", vector[0], vector[1]),
        |_index, _vector| Default::default(),
    );
    let x_idx = model.register_combination(&x)?;

    // 2. 成本符号组合
    let costs_ref = &data.costs;
    let x_idx_ref = &x_idx;
    let cost = flat_map1("cost", &data.workers, |w| {
        let w_idx = data.workers.iter().position(|ww| ww.name == w.name).unwrap();
        let monomials: Vec<_> = (0..data.tasks.len()).map(|t| {
            ospf_rust_core::symbol::flatten::LinearMonomial::new(costs_ref[w_idx][t], x_idx_ref[&[w_idx, t]])
        }).collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, w| w.name.clone());
    model.add_symbol_combination(&cost)?;

    // 3. 工人容量符号组合（每工人 sum(x[w,*]) <= 1）
    let worker_cap = flat_map1("worker_cap", &data.workers, |w| {
        let w_idx = data.workers.iter().position(|ww| ww.name == w.name).unwrap();
        let monomials: Vec<_> = (0..data.tasks.len()).map(|t| {
            ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx_ref[&[w_idx, t]])
        }).collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, w| w.name.clone());
    model.add_symbol_combination(&worker_cap)?;

    // 4. 任务分配符号组合（每任务 sum(x[*][t]) = 1）
    let task_part = flat_map1("task_part", &data.tasks, |t| {
        let t_idx = data.tasks.iter().position(|tt| tt.name == t.name).unwrap();
        let monomials: Vec<_> = (0..data.workers.len()).map(|w| {
            ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx_ref[&[w, t_idx]])
        }).collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, t| t.name.clone());
    model.add_symbol_combination(&task_part)?;

    // 5. 目标: 最小化总成本
    let cost_poly = cost[0].to_linear_polynomial();
    let cost_coeffs: Vec<_> = cost_poly.monomials().iter().map(|m| (m.var_index(), *m.coefficient())).collect();
    model.add_linear_objective(&cost_coeffs, "cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    // 6. 工人容量约束
    for (w, worker) in data.workers.iter().enumerate() {
        let poly = worker_cap[w].to_linear_polynomial();
        let coeffs: Vec<_> = poly.monomials().iter().map(|m| (m.var_index(), *m.coefficient())).collect();
        model.add_le_constraint_with_metadata(
            &coeffs, 1.0, &format!("worker_capacity_{}", worker.name),
            None, false, 1, Some("{\"kind\":\"worker-cap\"}".to_string()),
        )?;
    }

    // 7. 任务分配约束
    for (t, task) in data.tasks.iter().enumerate() {
        let poly = task_part[t].to_linear_polynomial();
        let coeffs: Vec<_> = poly.monomials().iter().map(|m| (m.var_index(), *m.coefficient())).collect();
        model.partition_linear_coefficients_with_metadata(
            &coeffs, &format!("task_partition_{}", task.name),
            None, false, 1, Some("{\"kind\":\"task-partition\"}".to_string()),
        )?;
    }

    // 8. 偏好约束
    model.add_ge_constraint_with_metadata(
        &[(x_idx[&[0, 0]], 1.0), (x_idx[&[1, 1]], 1.0)],
        1.0, "prefer_diagonal_shortcut", None, false, 2,
        Some("{\"kind\":\"preference\"}".to_string()),
    )?;

    Ok((model, x_idx))
}

/// 打印解决方案（MultiArray 版本）/ Print solution (MultiArray version)
fn print_solution(title: &str, data: &AssignmentData, idx: &MultiArray<usize, Shape<2>>, solution: &[f64]) {
    println!("{}", title);
    for (w, worker) in data.workers.iter().enumerate() {
        for (t, task) in data.tasks.iter().enumerate() {
            let value = read_solution_value(solution, idx[&[w, t]]);
            if value > 0.5 {
                println!("{} -> {}", worker.name, task.name);
            }
        }
    }
}

/// 打印解决方案（Vec 版本，legacy 路径用）/ Print solution (Vec version, for legacy path)
fn print_solution_vec(title: &str, data: &AssignmentData, idx: &[Vec<usize>], solution: &[f64]) {
    println!("{}", title);
    for (w, worker) in data.workers.iter().enumerate() {
        for (t, task) in data.tasks.iter().enumerate() {
            let value = read_solution_value(solution, idx[w][t]);
            if value > 0.5 {
                println!("{} -> {}", worker.name, task.name);
            }
        }
    }
}

/// Core shortcuts 主函数：演示低层与组合式建模对比
/// Core shortcuts main function: demonstrate low-level vs combination-based modeling comparison
pub fn run() -> Result<(), Box<dyn Error>> {
    let data = AssignmentData::sample();

    // 低层模型（legacy/teaching 路径）
    let (low_model, low_idx) = build_low_level_model(&data)?;

    // 组合式字段化模型（推荐主路径）
    let (shortcut_model, shortcut_idx) = build_shortcut_model(&data)?;

    let low_output = solve_typed(low_model)?;
    let low_solution = &low_output.solution;

    let shortcut_output = solve_typed(shortcut_model)?;
    let shortcut_solution = &shortcut_output.solution;

    println!("=== Core Shortcuts ===");
    println!("low-level status: {:?}", low_output.status);
    println!("shortcut status: {:?}", shortcut_output.status);
    if let Some(obj) = low_output.objective_value {
        println!("low-level objective: {:.2}", obj);
    }
    if let Some(obj) = shortcut_output.objective_value {
        println!("shortcut objective: {:.2}", obj);
    }

    print_solution_vec("low-level assignment:", &data, &low_idx, low_solution);
    print_solution("shortcut assignment:", &data, &shortcut_idx, shortcut_solution);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_shortcuts() {
        assert!(run().is_ok());
    }
}
