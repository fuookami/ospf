use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{UContinuousVariableItem, UIntegerVariableItem};

use super::common::{read_solution_value, solve};

#[derive(Debug, Clone)]
struct Dealer {
    name: String,
    demand: f64,
    distance_to_centers: Vec<f64>,
}

impl Dealer {
    fn new(name: &str, demand: f64, distance_to_centers: Vec<f64>) -> Self {
        Self {
            name: name.to_string(),
            demand,
            distance_to_centers,
        }
    }

    fn distance_to(&self, center_idx: usize) -> f64 {
        self.distance_to_centers[center_idx]
    }
}

#[derive(Debug, Clone)]
struct Center {
    name: String,
    supply: f64,
}

impl Center {
    fn new(name: &str, supply: f64) -> Self {
        Self {
            name: name.to_string(),
            supply,
        }
    }
}

fn build_dealers() -> Vec<Dealer> {
    vec![
        Dealer::new("D0", 100.0, vec![100.0, 50.0, 40.0]),
        Dealer::new("D1", 200.0, vec![150.0, 70.0, 90.0]),
        Dealer::new("D2", 150.0, vec![200.0, 60.0, 100.0]),
        Dealer::new("D3", 160.0, vec![140.0, 65.0, 150.0]),
        Dealer::new("D4", 140.0, vec![35.0, 80.0, 130.0]),
    ]
}

fn build_centers() -> Vec<Center> {
    vec![
        Center::new("C0", 400.0),
        Center::new("C1", 200.0),
        Center::new("C2", 150.0),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let dealers = build_dealers();
    let centers = build_centers();
    let car_capacity = 18.0;

    let mut model = MetaModel::<f64>::new("demo13");
    let mut x_idx = vec![vec![0usize; centers.len()]; dealers.len()];
    let mut y_idx = vec![vec![0usize; centers.len()]; dealers.len()];

    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            x_idx[d][c] = model
                .register_variable(UContinuousVariableItem::auto(&format!("x_{}_{}", d, c)))?;
            y_idx[d][c] =
                model.register_variable(UIntegerVariableItem::auto(&format!("y_{}_{}", d, c)))?;
        }
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            objective[y_idx[d][c]] = dealers[d].distance_to(c);
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for c in 0..centers.len() {
        let coefficients: Vec<(usize, f64)> =
            (0..dealers.len()).map(|d| (x_idx[d][c], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::LessEqual,
            centers[c].supply,
            &format!("supply_{}", c),
        )?;
    }

    for d in 0..dealers.len() {
        let coefficients: Vec<(usize, f64)> =
            (0..centers.len()).map(|c| (x_idx[d][c], 1.0)).collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::GreaterEqual,
            dealers[d].demand,
            &format!("demand_{}", d),
        )?;
    }

    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            model.add_linear_constraint(
                &[(x_idx[d][c], 1.0), (y_idx[d][c], -car_capacity)],
                ConstraintRelation::LessEqual,
                0.0,
                &format!("truck_{}_{}", d, c),
            )?;
        }
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo13 has no feasible solution"))?;

    println!("=== Demo13 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("transport cost: {:.2}", obj);
    }
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            let shipped = read_solution_value(&solution, x_idx[d][c]);
            if shipped > 0.0 {
                let trucks = read_solution_value(&solution, y_idx[d][c]);
                println!(
                    "{} <- {}: ship {:.2}, trucks {:.2}",
                    dealers[d].name, centers[c].name, shipped, trucks
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo13() {
        assert!(run().is_ok());
    }
}
