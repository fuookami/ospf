use std::error::Error;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{

    UContinuous, VariableCombination1D, VariableCombination3D, VariableRange,
};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_multiarray::{MultiArrayBuilder, Shape};

use super::common::{read_solution_value, solve_typed};

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
    let x_shape = Shape::new([manufacturers.len(), centers.len(), car_models.len()]);
    let x_vars: VariableCombination3D<UContinuous> =
        VariableCombination3D::with_name_and_range_generator(
            x_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}_{}", vector[0], vector[1], vector[2]),
            |_index, vector| {
                if manufacturers[vector[0]].productivity_by_model[vector[2]].is_some() {
                    VariableRange::with_lower(0.0)
                } else {
                    VariableRange::fixed(0.0)
                }
            },
        );
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<UContinuous, _>(x_vars.iter().cloned())?,
    );

    let mut y_vars: Vec<VariableCombination1D<UContinuous>> = Vec::with_capacity(centers.len());
    let mut y_idx = Vec::with_capacity(centers.len());
    for d in 0..centers.len() {
        let y_shape = Shape::new([centers[d].replacements.len()]);
        let y_for_center: VariableCombination1D<UContinuous> =
            VariableCombination1D::with_name_and_range_generator(
                y_shape.clone(),
                &format!("y_{}", d),
                |_index, vector| vector[0].to_string(),
                |_index, vector| {
                    VariableRange::bounded(0.0, centers[d].replacements[vector[0]].max_ratio)
                },
            );
        let y_indices = MultiArrayBuilder::from_list(
            y_shape,
            model.register_variables::<UContinuous, _>(y_for_center.iter().cloned())?,
        );
        y_vars.push(y_for_center);
        y_idx.push(y_indices);
    }

    let mut cost_terms = Vec::new();
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                cost_terms.push(LinearMonomial::new(
                    manufacturers[m].logistics_cost_to_centers[d],
                    x_vars[&[m, d, c]].to_owned_symbol(),
                ));
            }
        }
    }
    let cost = Linear::new(cost_terms, 0.0);
    let trans = MultiArrayBuilder::new_by(
        Shape::<2>::new([manufacturers.len(), car_models.len()]),
        |_idx, vec| {
            let m = vec[0];
            let c = vec[1];
            Linear::new(
                (0..centers.len())
                    .map(|d| LinearMonomial::new(1.0, x_vars[&[m, d, c]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        },
    );
    let receive = MultiArrayBuilder::new_by(
        Shape::<2>::new([centers.len(), car_models.len()]),
        |_idx, vec| {
            let d = vec[0];
            let c = vec[1];
            Linear::new(
                (0..manufacturers.len())
                    .map(|m| LinearMonomial::new(1.0, x_vars[&[m, d, c]].to_owned_symbol()))
                    .collect(),
                0.0,
            )
        },
    );
    let demand = MultiArrayBuilder::new_by(
        Shape::<2>::new([centers.len(), car_models.len()]),
        |_idx, vec| {
            let d = vec[0];
            let c = vec[1];
            let mut terms = Vec::new();
            for (r_idx, replacement) in centers[d].replacements.iter().enumerate() {
                if replacement.from == c {
                    terms.push(LinearMonomial::new(
                        centers[d].demands[replacement.from],
                        y_vars[d][r_idx].to_owned_symbol(),
                    ));
                }
                if replacement.to == c {
                    terms.push(LinearMonomial::new(
                        -centers[d].demands[replacement.from],
                        y_vars[d][r_idx].to_owned_symbol(),
                    ));
                }
            }
            Linear::new(terms, 0.0)
        },
    );

    model.set_math_linear_objective(cost, ObjectiveCategory::Minimum, "cost")?;

    for d in 0..centers.len() {
        for c in 0..car_models.len() {
            model.add_math_inequality(
                (receive[&[d, c]].clone() + demand[&[d, c]].clone()).ge(centers[d].demands[c]),
                &format!("demand_{}_{}", d, c),
            );
        }
    }

    for m in 0..manufacturers.len() {
        for c in 0..car_models.len() {
            if let Some(cap) = manufacturers[m].productivity_by_model[c] {
                model.add_math_inequality(
                    trans[&[m, c]].clone().le(cap),
                    &format!("capacity_{}_{}", m, c),
                );
            }
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo15 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("cost: {:.2}", obj);
    }
    for m in 0..manufacturers.len() {
        for d in 0..centers.len() {
            for c in 0..car_models.len() {
                let value = read_solution_value(&solution, x_idx[&[m, d, c]]);
                if value > 0.0 {
                    println!(
                        "{} -> {} {} = {:.2}",
                        manufacturers[m].name, centers[d].name, car_models[c].name, value
                    );
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
