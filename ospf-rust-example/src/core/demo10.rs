use std::error::Error;

use ospf_rust_core::model::object::ObjectiveCategory;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::{BinaryVariableItem, IntegerVariableItem, VariableRange};

use super::common::{read_solution_value, solve};

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

#[derive(Debug, Clone)]
struct DistanceMatrix {
    values: Vec<Vec<f64>>,
}

impl DistanceMatrix {
    fn new(values: Vec<Vec<f64>>) -> Self {
        Self { values }
    }

    fn get(&self, from: usize, to: usize) -> f64 {
        self.values[from][to]
    }
}

#[derive(Debug, Clone)]
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
            distances: DistanceMatrix::new(vec![
                vec![0.0, 472.0, 1520.0, 2095.0, 1244.0],
                vec![472.0, 0.0, 1257.0, 1615.0, 1044.0],
                vec![1529.0, 1257.0, 0.0, 1954.0, 2174.0],
                vec![2095.0, 1615.0, 1954.0, 0.0, 1854.0],
                vec![1244.0, 1044.0, 2174.0, 1854.0, 0.0],
            ]),
        }
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let data = TspData::sample();
    let n = data.cities.len() as f64;

    let mut model = MetaModel::<f64>::new("demo10");
    let mut x_idx = vec![vec![None; data.cities.len()]; data.cities.len()];
    for i in 0..data.cities.len() {
        for j in 0..data.cities.len() {
            if i == j {
                continue;
            }
            let var = BinaryVariableItem::auto(&format!("x_{}_{}", i, j));
            x_idx[i][j] = Some(model.register_variable(var)?);
        }
    }

    let mut u_idx = vec![0usize; data.cities.len()];
    for i in 0..data.cities.len() {
        let var = IntegerVariableItem::auto_with_range(
            &format!("u_{}", i),
            VariableRange::bounded(-(data.cities.len() as f64), data.cities.len() as f64),
        );
        u_idx[i] = model.register_variable(var)?;
    }
    model.add_linear_constraint(
        &[(u_idx[data.begin_idx], 1.0)],
        ConstraintRelation::Equal,
        0.0,
        "u_begin",
    )?;

    let mut objective = vec![0.0; model.num_tokens()];
    for i in 0..data.cities.len() {
        for j in 0..data.cities.len() {
            if let Some(idx) = x_idx[i][j] {
                objective[idx] = data.distances.get(i, j);
            }
        }
    }
    model.set_linear_objective(objective, ObjectiveCategory::Minimum);

    for i in 0..data.cities.len() {
        let coefficients: Vec<(usize, f64)> = (0..data.cities.len())
            .filter_map(|j| x_idx[i][j].map(|idx| (idx, 1.0)))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("depart_{}", i),
        )?;
    }

    for j in 0..data.cities.len() {
        let coefficients: Vec<(usize, f64)> = (0..data.cities.len())
            .filter_map(|i| x_idx[i][j].map(|idx| (idx, 1.0)))
            .collect();
        model.add_linear_constraint(
            &coefficients,
            ConstraintRelation::Equal,
            1.0,
            &format!("arrive_{}", j),
        )?;
    }

    for i in 0..data.cities.len() {
        if i == data.begin_idx {
            continue;
        }
        for j in 0..data.cities.len() {
            if j == data.begin_idx || i == j {
                continue;
            }
            if let Some(xij) = x_idx[i][j] {
                model.add_linear_constraint(
                    &[(u_idx[i], 1.0), (u_idx[j], -1.0), (xij, n)],
                    ConstraintRelation::LessEqual,
                    n - 1.0,
                    &format!("mtz_{}_{}", i, j),
                )?;
            }
        }
    }

    let output = solve(model)?;
    let solution = output
        .solution
        .ok_or_else(|| String::from("demo10 has no feasible solution"))?;

    println!("=== Demo10 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("distance: {:.2}", obj);
    }
    for i in 0..data.cities.len() {
        for j in 0..data.cities.len() {
            if let Some(idx) = x_idx[i][j] {
                if read_solution_value(&solution, idx) > 0.5 {
                    println!("{} -> {}", data.cities[i].name, data.cities[j].name);
                }
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
