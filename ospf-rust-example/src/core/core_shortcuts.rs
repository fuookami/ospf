use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel, SymbolicLinearInequality};
use ospf_rust_core::variable::BinaryVariableItem;
use ospf_rust_math::symbol::{Linear, LinearMonomial};

use super::common::{read_solution_value, solve};

#[derive(Clone)]
struct Worker {
    name: String,
}

impl Worker {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Clone)]
struct Task {
    name: String,
}

impl Task {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Clone)]
struct AssignmentData {
    workers: Vec<Worker>,
    tasks: Vec<Task>,
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

fn register_assignment_vars(
    model: &mut MetaModel<f64>,
    data: &AssignmentData,
) -> Result<(Vec<Vec<BinaryVariableItem>>, Vec<Vec<usize>>), Box<dyn Error>> {
    let mut vars: Vec<Vec<BinaryVariableItem>> = Vec::with_capacity(data.workers.len());
    let mut idx: Vec<Vec<usize>> = Vec::with_capacity(data.workers.len());

    for w in 0..data.workers.len() {
        let mut var_row: Vec<BinaryVariableItem> = Vec::with_capacity(data.tasks.len());
        let mut idx_row: Vec<usize> = Vec::with_capacity(data.tasks.len());
        for t in 0..data.tasks.len() {
            let var = BinaryVariableItem::auto(&format!("x_{}_{}", w, t));
            let token_idx = model.register_variable(var.clone())?;
            var_row.push(var);
            idx_row.push(token_idx);
        }
        vars.push(var_row);
        idx.push(idx_row);
    }

    Ok((vars, idx))
}

fn set_objective(model: &mut MetaModel<f64>, data: &AssignmentData, idx: &[Vec<usize>]) {
    let mut objective = vec![0.0; model.num_tokens()];
    for w in 0..data.workers.len() {
        for t in 0..data.tasks.len() {
            objective[idx[w][t]] = data.costs[w][t];
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);
}

fn build_low_level_model(data: &AssignmentData) -> Result<(MetaModel<f64>, Vec<Vec<usize>>), Box<dyn Error>> {
    let mut model = MetaModel::<f64>::new("core_shortcuts_low_level");
    let (_vars, idx) = register_assignment_vars(&mut model, data)?;
    set_objective(&mut model, data, &idx);

    for (w, worker) in data.workers.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = (0..data.tasks.len()).map(|t| (idx[w][t], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            1.0,
            &format!("worker_capacity_{}", worker.name),
        )?;
    }

    for (t, task) in data.tasks.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = (0..data.workers.len()).map(|w| (idx[w][t], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("task_partition_{}", task.name),
        )?;
    }

    model.add_linear_constraint(
        &[(idx[0][0], 1.0), (idx[1][1], 1.0)],
        ConstraintRelation::GreaterEqual,
        1.0,
        "prefer_diagonal_low_level",
    )?;

    Ok((model, idx))
}

fn build_shortcut_model(data: &AssignmentData) -> Result<(MetaModel<f64>, Vec<Vec<usize>>), Box<dyn Error>> {
    let mut model = MetaModel::<f64>::new("core_shortcuts_shortcut");
    let (vars, idx) = register_assignment_vars(&mut model, data)?;
    set_objective(&mut model, data, &idx);

    let group = model.create_constraint_group(1801, "assignment_shortcuts")?;

    for (w, worker) in data.workers.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = (0..data.tasks.len()).map(|t| (idx[w][t], 1.0)).collect();
        model.add_le_constraint_with_metadata(
            &coefficients,
            1.0,
            &format!("worker_capacity_{}", worker.name),
            Some(group.clone()),
            false,
            1,
            Some("{\"kind\":\"worker-cap\"}".to_string()),
        )?;
    }

    let task0_coefficients: Vec<(usize, f64)> = (0..data.workers.len()).map(|w| (idx[w][0], 1.0)).collect();
    model.partition_linear_coefficients_with_metadata(
        &task0_coefficients,
        &format!("task_partition_{}", data.tasks[0].name),
        Some(group.clone()),
        false,
        1,
        Some("{\"kind\":\"task-partition\"}".to_string()),
    )?;

    let task1_indices: Vec<usize> = (0..data.workers.len()).map(|w| idx[w][1]).collect();
    model.partition_linear_indices_with_metadata(
        &task1_indices,
        &format!("task_partition_{}", data.tasks[1].name),
        Some(group.clone()),
        false,
        1,
        Some("{\"kind\":\"task-partition\"}".to_string()),
    )?;

    model.add_ge_constraint_with_metadata(
        &[(idx[0][0], 1.0), (idx[1][1], 1.0)],
        1.0,
        "prefer_diagonal_shortcut",
        Some(group),
        false,
        2,
        Some("{\"kind\":\"preference\"}".to_string()),
    )?;

    let symbolic = vec![
        (
            SymbolicLinearInequality::greater_equal(
                Linear::new(
                    vec![
                        LinearMonomial::new(1.0, vars[0][0].to_owned_symbol()),
                        LinearMonomial::new(1.0, vars[1][1].to_owned_symbol()),
                    ],
                    0.0,
                ),
                1.0,
            ),
            "prefer_diagonal_symbolic",
        ),
    ];
    model.add_symbolic_inequalities(symbolic);

    Ok((model, idx))
}

fn print_solution(title: &str, data: &AssignmentData, idx: &[Vec<usize>], solution: &[f64]) {
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

pub fn run() -> Result<(), Box<dyn Error>> {
    let data = AssignmentData::sample();

    let (low_model, low_idx) = build_low_level_model(&data)?;
    let (shortcut_model, shortcut_idx) = build_shortcut_model(&data)?;

    let low_output = solve(low_model)?;
    let low_solution = low_output
        .solution
        .as_ref()
        .ok_or_else(|| String::from("core_shortcuts low-level has no feasible solution"))?;

    let shortcut_output = solve(shortcut_model)?;
    let shortcut_solution = shortcut_output
        .solution
        .as_ref()
        .ok_or_else(|| String::from("core_shortcuts shortcut has no feasible solution"))?;

    println!("=== Core Shortcuts ===");
    println!("low-level status: {:?}", low_output.status);
    println!("shortcut status: {:?}", shortcut_output.status);
    if let Some(obj) = low_output.objective_value {
        println!("low-level objective: {:.2}", obj);
    }
    if let Some(obj) = shortcut_output.objective_value {
        println!("shortcut objective: {:.2}", obj);
    }

    print_solution("low-level assignment:", &data, &low_idx, low_solution);
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
