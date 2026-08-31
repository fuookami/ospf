use std::error::Error;

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{UContinuous, UInteger, VariableCombination2D};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};

use super::common::{read_solution_value, solve_typed};

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
    let variable_shape = Shape::new([dealers.len(), centers.len()]);
    let x_vars: VariableCombination2D<UContinuous> = VariableCombination2D::with_name_generator(
        variable_shape.clone(),
        "x",
        |_index, vector| format!("{}_{}", vector[0], vector[1]),
    );
    let y_vars: VariableCombination2D<UInteger> = VariableCombination2D::with_name_generator(
        variable_shape.clone(),
        "y",
        |_index, vector| format!("{}_{}", vector[0], vector[1]),
    );
    let x_idx = MultiArrayBuilder::from_list(
        variable_shape.clone(),
        model.register_variables::<UContinuous, _>(x_vars.iter().cloned())?,
    );
    let y_idx = MultiArrayBuilder::from_list(
        variable_shape,
        model.register_variables::<UInteger, _>(y_vars.iter().cloned())?,
    );

    let mut cost_terms = Vec::with_capacity(dealers.len() * centers.len());
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            cost_terms.push(LinearMonomial::new(
                dealers[d].distance_to(c),
                y_vars[&[d, c]].to_owned_symbol(),
            ));
        }
    }
    let cost = Linear::new(cost_terms, 0.0);
    let trans = MultiArrayBuilder::new_by(Shape::<1>::new([centers.len()]), |_idx, vec| {
        let c = vec[0];
        Linear::new(
            dealers
                .iter()
                .enumerate()
                .map(|(d, _)| LinearMonomial::new(1.0, x_vars[&[d, c]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let receive = MultiArrayBuilder::new_by(Shape::<1>::new([dealers.len()]), |_idx, vec| {
        let d = vec[0];
        Linear::new(
            centers
                .iter()
                .enumerate()
                .map(|(c, _)| LinearMonomial::new(1.0, x_vars[&[d, c]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(cost, ObjectiveCategory::Minimum, "cost")?;

    for c in 0..centers.len() {
        model.add_math_inequality(
            trans[c].clone().le(centers[c].supply),
            &format!("supply_{}", c),
        );
    }

    for d in 0..dealers.len() {
        model.add_math_inequality(
            receive[d].clone().ge(dealers[d].demand),
            &format!("demand_{}", d),
        );
    }

    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            let truck = Linear::new(
                vec![
                    LinearMonomial::new(1.0, x_vars[&[d, c]].to_owned_symbol()),
                    LinearMonomial::new(-car_capacity, y_vars[&[d, c]].to_owned_symbol()),
                ],
                0.0,
            );
            model.add_math_inequality(truck.le(0.0), &format!("truck_{}_{}", d, c));
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo13 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("transport cost: {:.2}", obj);
    }
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            let shipped = read_solution_value(&solution, x_idx[&[d, c]]);
            if shipped > 0.0 {
                let trucks = read_solution_value(&solution, y_idx[&[d, c]]);
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
