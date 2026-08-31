use std::error::Error;
use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::variable::{

    Binary, Integer, VariableCombination1D, VariableCombination2D, VariableRange,
};
use ospf_rust_math::symbol::{Linear, LinearMonomial};
use ospf_rust_multiarray::{MultiArray, MultiArrayBuilder, Shape};

use super::common::{read_solution_value, solve_typed};

#[derive(Debug, Clone)]
struct City {
    name: String,
}

impl City {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[derive(Clone)]
struct DistanceMatrix {
    values: MultiArray<f64, Shape<2>>,
}

impl DistanceMatrix {
    fn new(size: usize, values: Vec<f64>) -> Self {
        Self {
            values: MultiArrayBuilder::from_list(Shape::<2>::new([size, size]), values),
        }
    }

    fn get(&self, from: usize, to: usize) -> f64 {
        self.values[&[from, to]]
    }
}

#[derive(Clone)]
struct TspData {
    cities: Vec<City>,
    begin_idx: usize,
    distances: DistanceMatrix,
}

impl TspData {
    fn sample() -> Self {
        Self {
            cities: vec![
                City::new("Shanghai"),
                City::new("Hefei"),
                City::new("Guangzhou"),
                City::new("Chengdu"),
                City::new("Beijing"),
            ],
            begin_idx: 4,
            distances: DistanceMatrix::new(
                5,
                vec![
                    0.0, 472.0, 1520.0, 2095.0, 1244.0, 472.0, 0.0, 1257.0, 1615.0, 1044.0, 1529.0,
                    1257.0, 0.0, 1954.0, 2174.0, 2095.0, 1615.0, 1954.0, 0.0, 1854.0, 1244.0,
                    1044.0, 2174.0, 1854.0, 0.0,
                ],
            ),
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let data = TspData::sample();
    let n = data.cities.len() as f64;

    let mut model = MetaModel::<f64>::new("demo10");
    let city_count = data.cities.len();
    let x_shape = Shape::new([city_count, city_count]);
    let x_vars: VariableCombination2D<Binary> =
        VariableCombination2D::with_name_and_range_generator(
            x_shape.clone(),
            "x",
            |_index, vector| format!("{}_{}", vector[0], vector[1]),
            |_index, vector| {
                if vector[0] == vector[1] {
                    VariableRange::fixed(0.0)
                } else {
                    VariableRange::bounded(0.0, 1.0)
                }
            },
        );
    let x_idx = MultiArrayBuilder::from_list(
        x_shape,
        model.register_variables::<Binary, _>(x_vars.iter().cloned())?,
    );

    let u_shape = Shape::new([city_count]);
    let u_vars: VariableCombination1D<Integer> =
        VariableCombination1D::with_name_and_range_generator(
            u_shape.clone(),
            "u",
            |_index, vector| vector[0].to_string(),
            |_index, vector| {
                if vector[0] == data.begin_idx {
                    VariableRange::fixed(0.0)
                } else {
                    VariableRange::bounded(-(city_count as f64), city_count as f64)
                }
            },
        );
    model.register_variables::<Integer, _>(u_vars.iter().cloned())?;

    let mut distance_terms = Vec::new();
    for i in 0..city_count {
        for j in 0..city_count {
            distance_terms.push(LinearMonomial::new(
                data.distances.get(i, j),
                x_vars[&[i, j]].to_owned_symbol(),
            ));
        }
    }
    let distance = Linear::new(distance_terms, 0.0);
    let depart = MultiArrayBuilder::new_by(Shape::<1>::new([city_count]), |_idx, vector| {
        let i = vector[0];
        Linear::new(
            (0..city_count)
                .map(|j| LinearMonomial::new(1.0, x_vars[&[i, j]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });
    let reached = MultiArrayBuilder::new_by(Shape::<1>::new([city_count]), |_idx, vector| {
        let j = vector[0];
        Linear::new(
            (0..city_count)
                .map(|i| LinearMonomial::new(1.0, x_vars[&[i, j]].to_owned_symbol()))
                .collect(),
            0.0,
        )
    });

    model.set_math_linear_objective(distance, ObjectiveCategory::Minimum, "distance")?;

    for i in 0..city_count {
        model.add_math_inequality(depart[i].clone().eq_to(1.0), &format!("depart_{}", i));
    }

    for j in 0..city_count {
        model.add_math_inequality(reached[j].clone().eq_to(1.0), &format!("arrive_{}", j));
    }

    for i in 0..city_count {
        if i == data.begin_idx {
            continue;
        }
        for j in 0..city_count {
            if j == data.begin_idx || i == j {
                continue;
            }
            let mtz = Linear::new(
                vec![
                    LinearMonomial::new(1.0, u_vars[i].to_owned_symbol()),
                    LinearMonomial::new(-1.0, u_vars[j].to_owned_symbol()),
                    LinearMonomial::new(n, x_vars[&[i, j]].to_owned_symbol()),
                ],
                0.0,
            );
            model.add_math_inequality(mtz.le(n - 1.0), &format!("mtz_{}_{}", i, j));
        }
    }

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo10 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("distance: {:.2}", obj);
    }
    for i in 0..city_count {
        for j in 0..city_count {
            if read_solution_value(&solution, x_idx[&[i, j]]) > 0.5 {
                println!("{} -> {}", data.cities[i].name, data.cities[j].name);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo10() {
        assert!(run().is_ok());
    }
}
