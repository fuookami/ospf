use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory, ConstraintRelation};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, LinearIntermediateSymbol, flat_map1,
};
use ospf_rust_core::variable::{UContinuous, UInteger, VariableCombination2D};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// Dealer data structure
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

/// Distribution center data structure
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

/// Build dealer list
fn build_dealers() -> Vec<Dealer> {
    vec![
        Dealer::new("D0", 100.0, vec![100.0, 50.0, 40.0]),
        Dealer::new("D1", 200.0, vec![150.0, 70.0, 90.0]),
        Dealer::new("D2", 150.0, vec![200.0, 60.0, 100.0]),
        Dealer::new("D3", 160.0, vec![140.0, 65.0, 150.0]),
        Dealer::new("D4", 140.0, vec![35.0, 80.0, 130.0]),
    ]
}

/// Build distribution center list
fn build_centers() -> Vec<Center> {
    vec![
        Center::new("C0", 400.0),
        Center::new("C1", 200.0),
        Center::new("C2", 150.0),
    ]
}

/// Demo13 main function: Vehicle delivery problem
pub fn run() -> Result<(), Box<dyn Error>> {
    let dealers = build_dealers();
    let centers = build_centers();
    let car_capacity = 18.0;

    let mut model = MetaModel::<f64>::new("demo13");

    // Register 2D variable combinations
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
    let x_idx = model.register_combination(&x_vars)?;
    let y_idx = model.register_combination(&y_vars)?;

    // Objective: minimize cost = sum(distance[d][c] * y[d][c])
    let cost_expr = flat_map1("cost", &dealers, |dealer| {
        let d = dealers.iter().position(|dd| dd.name == dealer.name).unwrap();
        let monomials: Vec<_> = (0..centers.len())
            .map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                dealer.distance_to(c),
                y_idx[&[d, c]],
            ))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, dealer| dealer.name.clone());
    model.add_symbol_combination(&cost_expr)?;

    // Aggregate cost objective
    let mut cost_coeffs = Vec::new();
    for d in 0..dealers.len() {
        let poly = cost_expr.symbol_polynomial(d);
        for m in poly.monomials() {
            cost_coeffs.push((m.var_index(), *m.coefficient()));
        }
    }
    model.add_linear_objective(&cost_coeffs, "cost");
    model.set_objective_category(ObjectiveCategory::Minimum);

    // Supply constraints per center: sum_d x[d][c] <= supply[c]
    let trans_expr = flat_map1("trans", &centers, |center| {
        let c = centers.iter().position(|cc| cc.name == center.name).unwrap();
        let monomials: Vec<_> = (0..dealers.len())
            .map(|d| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[d, c]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, center| center.name.clone());
    model.add_symbol_combination(&trans_expr)?;

    for c in 0..centers.len() {
        let coeffs = extract_coeffs(&trans_expr[c]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::LessEqual,
            centers[c].supply,
            &format!("supply_{}", c),
        )?;
    }

    // Demand constraints per dealer: sum_c x[d][c] >= demand[d]
    let receive_expr = flat_map1("receive", &dealers, |dealer| {
        let d = dealers.iter().position(|dd| dd.name == dealer.name).unwrap();
        let monomials: Vec<_> = (0..centers.len())
            .map(|c| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[d, c]]))
            .collect();
        ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
    }, |_, dealer| dealer.name.clone());
    model.add_symbol_combination(&receive_expr)?;

    for d in 0..dealers.len() {
        let coeffs = extract_coeffs(&receive_expr[d]);
        model.add_linear_constraint(
            &coeffs,
            ConstraintRelation::GreaterEqual,
            dealers[d].demand,
            &format!("demand_{}", d),
        )?;
    }

    // Truck capacity constraints: x[d][c] - capacity * y[d][c] <= 0
    for d in 0..dealers.len() {
        for c in 0..centers.len() {
            let truck = ospf_rust_core::symbol::flatten::Linear::new(
                vec![
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[d, c]]),
                    ospf_rust_core::symbol::flatten::LinearMonomial::new(-car_capacity, y_idx[&[d, c]]),
                ],
                0.0,
            );
            let coeffs: Vec<_> = truck.monomials().iter()
                .map(|m| (m.var_index(), *m.coefficient())).collect();
            model.add_linear_constraint(
                &coeffs,
                ConstraintRelation::LessEqual,
                0.0,
                &format!("truck_{}_{}", d, c),
            )?;
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
