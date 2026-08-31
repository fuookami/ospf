use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UIntegerVariableItem;

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Warehouse {
    name: String,
    stowage: f64,
    costs_to_stores: Vec<f64>,
}

impl Warehouse {
    fn new(name: &str, stowage: f64, costs_to_stores: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            stowage,
            costs_to_stores,
        }
    }

    fn cost_to(&self, store_idx: usize) -> f64 {
        self.costs_to_stores[store_idx]
    }
}

#[derive(Debug, Clone)]
struct Store {
    name: String,
    demand: f64,
}

impl Store {
    fn new(name: &str, demand: f64) -> Self {
        Self {
            name: name.to_string(),
            demand,
        }
    }
}

fn build_warehouses() -> Vec<Warehouse> {
    vec![
        Warehouse::new("W0", 510.0, vec![12.0, 13.0, 21.0, 7.0]),
        Warehouse::new("W1", 470.0, vec![14.0, 17.0, 8.0, 18.0]),
        Warehouse::new("W2", 520.0, vec![10.0, 11.0, 9.0, 15.0]),
    ]
}

fn build_stores() -> Vec<Store> {
    vec![
        Store::new("S0", 200.0),
        Store::new("S1", 400.0),
        Store::new("S2", 600.0),
        Store::new("S3", 300.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let warehouses = build_warehouses();
    let stores = build_stores();

    let mut model = MetaModel::<f64>::new("demo7");
    let mut x_idx = vec![vec![0usize; stores.len()]; warehouses.len()];

    for (w, _) in warehouses.iter().enumerate() {
        for (s, _) in stores.iter().enumerate() {
            let variable = UIntegerVariableItem::auto(&format!("x_{}_{}", w, s));
            x_idx[w][s] = model.register_variable(variable)?;
        }
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, _) in stores.iter().enumerate() {
            objective[x_idx[w][s]] = warehouse.cost_to(s);
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for (w, _) in warehouses.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = stores
            .iter()
            .enumerate()
            .map(|(s, _)| (x_idx[w][s], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            warehouses[w].stowage,
            &format!("stowage_{}", w),
        )?;
    }

    for (s, _) in stores.iter().enumerate() {
        let coefficients: Vec<(usize, f64)> = warehouses
            .iter()
            .enumerate()
            .map(|(w, _)| (x_idx[w][s], 1.0))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            stores[s].demand,
            &format!("demand_{}", s),
        )?;
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo7 has no feasible solution"))?;

    println!("=== Demo7 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for (w, warehouse) in warehouses.iter().enumerate() {
        for (s, store) in stores.iter().enumerate() {
            let value = read_solution_value(&solution, x_idx[w][s]);
            if value >= 1.0 {
                println!("{} -> {} = {:.2}", warehouse.name, store.name, value);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo7() {
        assert!(run().is_ok());
    }
}
