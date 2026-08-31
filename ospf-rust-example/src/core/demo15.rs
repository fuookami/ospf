use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{UContinuousVariableItem, VariableRange};

use super::common::{read_solution_value, solve};

#[derive(Clone, Copy)]
struct Replacement {
    from: usize,
    to: usize,
    max_ratio: f64,
}

#[derive(Clone)]
struct CarModel {
    name: String,
}

impl CarModel {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Clone)]
struct Center {
    name: String,
    demands: Vec<f64>,
    replacements: Vec<Replacement>,
}

impl Center {
    fn new(name: &str, demands: Vec<f64>, replacements: Vec<Replacement>) -> Self {
        Self {
            name: name.to_string(),
            demands,
            replacements,
        }
    }
}

#[derive(Clone)]
struct Manufacturer {
    name: String,
    productivity_by_model: Vec<Option<f64>>,
    logistics_cost_to_centers: Vec<f64>,
}

impl Manufacturer {
    fn new(
        name: &str,
        productivity_by_model: Vec<Option<f64>>,
        logistics_cost_to_centers: Vec<f64>,
    ) -> Self {
        Self {
            name: name.to_string(),
            productivity_by_model,
            logistics_cost_to_centers,
        }
    }
}

fn build_car_models() -> Vec<CarModel> {
    vec![
        CarModel::new("M1"),
        CarModel::new("M2"),
        CarModel::new("M3"),
        CarModel::new("M4"),
    ]
}

fn build_centers() -> Vec<Center> {
    vec![
        Center::new(
            "Denver",
            vec![700.0, 500.0, 500.0, 600.0],
            vec![
                Replacement {
                    from: 0,
                    to: 1,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 0,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 2,
                    to: 3,
                    max_ratio: 0.20,
                },
                Replacement {
                    from: 3,
                    to: 2,
                    max_ratio: 0.20,
                },
            ],
        ),
        Center::new(
            "Miami",
            vec![600.0, 500.0, 200.0, 100.0],
            vec![
                Replacement {
                    from: 0,
                    to: 1,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 0,
                    max_ratio: 0.10,
                },
                Replacement {
                    from: 1,
                    to: 3,
                    max_ratio: 0.05,
                },
                Replacement {
                    from: 3,
                    to: 1,
                    max_ratio: 0.05,
                },
            ],
        ),
    ]
}

fn build_manufacturers() -> Vec<Manufacturer> {
    vec![
        Manufacturer::new(
            "LosAngeles",
            vec![None, None, Some(700.0), Some(300.0)],
            vec![80.0, 215.0],
        ),
        Manufacturer::new(
            "Detroit",
            vec![Some(500.0), Some(600.0), None, Some(400.0)],
            vec![100.0, 108.0],
        ),
        Manufacturer::new(
            "NewOrleans",
            vec![Some(800.0), Some(400.0), None, None],
            vec![102.0, 68.0],
        ),
    ]
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let car_models = build_car_models();
    let centers = build_centers();
    let manufacturers = build_manufacturers();

    let mut model = MetaModel::<f64>::new("demo15");
    let mut x_idx = vec![vec![vec![None; car_models.len()]; centers.len()]; manufacturers.len()];
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                if manufacturers[m].productivity_by_model[c].is_some() {
                    let var = UContinuousVariableItem::auto(&format!("x_{}_{}_{}", m, d, c));
                    x_idx[m][d][c] = Some(model.register_variable(var)?);
                }
            }
        }
    }

    let mut y_idx = vec![Vec::<usize>::new(); centers.len()];
    for d in 0..centers.len() {
        for (r, replacement) in centers[d].replacements.iter().enumerate() {
            let var = UContinuousVariableItem::auto_with_range(
                &format!("y_{}_{}", d, r),
                VariableRange::bounded(0.0, replacement.max_ratio),
            );
            y_idx[d].push(model.register_variable(var)?);
        }
    }

    let mut objective = vec![0.0; model.num_tokens()];
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                if let Some(idx) = x_idx[m][d][c] {
                    objective[idx] = manufacturers[m].logistics_cost_to_centers[d];
                }
            }
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for d in 0..centers.len() {
        for c in 0..car_models.len() {
            let mut coefficients: Vec<(usize, f64)> = Vec::new();
            for m in 0..manufacturers.len() {
                if let Some(idx) = x_idx[m][d][c] {
                    coefficients.push((idx, 1.0));
                }
            }
            for (r_idx, replacement) in centers[d].replacements.iter().enumerate() {
                let y = y_idx[d][r_idx];
                if replacement.from == c {
                    coefficients.push((y, centers[d].demands[replacement.from]));
                }
                if replacement.to == c {
                    coefficients.push((y, -centers[d].demands[replacement.from]));
                }
            }
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                centers[d].demands[c],
                &format!("demand_{}_{}", d, c),
            )?;
        }
    }

    for m in 0..manufacturers.len() {
        for c in 0..car_models.len() {
            if let Some(cap) = manufacturers[m].productivity_by_model[c] {
                let coefficients: Vec<(usize, f64)> = (0..centers.len())
                    .filter_map(|d| x_idx[m][d][c].map(|idx| (idx, 1.0)))
                    .collect();
                model.add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::LessEqual,
                    cap,
                    &format!("capacity_{}_{}", m, c),
                )?;
            }
        }
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo15 has no feasible solution"))?;

    println!("=== Demo15 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                if let Some(idx) = x_idx[m][d][c] {
                    let value = read_solution_value(&solution, idx);
                    if value > 0.0 {
                        println!(
                            "{} -> {} {} = {:.2}",
                            manufacturers[m].name, centers[d].name, car_models[c].name, value
                        );
                    }
                }
            }
        }
    }
    for d in 0..centers.len() {
        for (r, replacement) in centers[d].replacements.iter().enumerate() {
            let ratio = read_solution_value(&solution, y_idx[d][r]);
            if ratio > 0.0 {
                println!(
                    "{} replace {} -> {} ratio {:.4}",
                    centers[d].name,
                    car_models[replacement.from].name,
                    car_models[replacement.to].name,
                    ratio
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
    fn test_demo15() {
        assert!(run().is_ok());
    }
}
