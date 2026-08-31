use std::error::Error;

use ospf_rust_multiarray::Shape;
use ospf_rust_core::model::{ConstraintRelation, LinearObjectiveInput, MetaModel};
use ospf_rust_core::symbol::{
    LinearExpressionSymbol, flat_map1_indexed, flat_map2_indexed,
};
use ospf_rust_core::variable::{Binary, Integer, VariableCombination1D, VariableCombination2D, VariableRange};

use super::common::{read_solution_value, solve_typed, extract_coeffs};

/// 城市数据结构 / City data structure
#[derive(Debug, Clone)]
struct City {
    /// 城市名称 / City name
    name: String,
}

impl City {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// 距离矩阵 / Distance matrix
#[derive(Clone)]
struct DistanceMatrix {
    /// 距离值 / Distance values
    values: Vec<Vec<f64>>,
}

impl DistanceMatrix {
    fn new(size: usize, flat: Vec<f64>) -> Self {
        let mut values = Vec::with_capacity(size);
        for i in 0..size {
            let row: Vec<f64> = flat[i * size..(i + 1) * size].to_vec();
            values.push(row);
        }
        Self { values }
    }

    /// 获取两点间距离 / Get distance between two points
    fn get(&self, from: usize, to: usize) -> f64 {
        self.values[from][to]
    }
}

/// TSP 数据结构 / TSP data structure
#[derive(Clone)]
struct TspData {
    /// 城市列表 / City list
    cities: Vec<City>,
    /// 起始城市索引 / Begin city index
    begin_idx: usize,
    /// 距离矩阵 / Distance matrix
    distances: DistanceMatrix,
}

impl TspData {
    /// 示例数据 / Sample data
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

/// 旅行商问题模型 / TSP model
///
/// 所有变量和中间符号都是显式字段。
/// All variables and intermediate symbols are explicit fields.
struct TspModel {
    /// 路径决策变量 / Path decision variables
    x_vars: VariableCombination2D<Binary>,
    /// 路径模型索引数组 / Path model index array
    x_idx: ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
    /// MTZ 辅助变量 / MTZ auxiliary variables
    u_vars: VariableCombination1D<Integer>,
    /// MTZ 模型索引数组 / MTZ model index array
    u_idx: ospf_rust_multiarray::MultiArray<usize, Shape<1>>,
    /// 距离目标符号 / Distance objective symbol
    distance: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 出发约束符号 / Depart constraint symbol
    depart: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// 到达约束符号 / Arrive constraint symbol
    reached: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    /// MTZ 子回路消除符号 / MTZ subtour elimination symbol
    mtz: ospf_rust_core::symbol::SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<2>>,
}

impl TspModel {
    /// 注册模型 / Register model
    fn register(
        model: &mut MetaModel<f64>,
        data: &TspData,
    ) -> Result<Self, Box<dyn Error>> {
        let n = data.cities.len() as f64;
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
        let x_idx = model.register_combination(&x_vars)?;

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
        let u_idx = model.register_combination(&u_vars)?;

        // 距离目标符号 / Distance objective symbol
        let distance = flat_map1_indexed("distance", &data.cities, |i, _city| {
            let monomials: Vec<_> = (0..city_count)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(
                    data.distances.get(i, j), x_idx[&[i, j]],
                ))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, city| city.name.clone());
        model.add_symbol_combination(&distance)?;

        // 出发约束符号 / Depart constraint symbol
        let depart = flat_map1_indexed("depart", &data.cities, |i, _city| {
            let monomials: Vec<_> = (0..city_count)
                .map(|j| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, city| city.name.clone());
        model.add_symbol_combination(&depart)?;

        // 到达约束符号 / Arrive constraint symbol
        let reached = flat_map1_indexed("reached", &data.cities, |j, _city| {
            let monomials: Vec<_> = (0..city_count)
                .map(|i| ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, x_idx[&[i, j]]))
                .collect();
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, city| city.name.clone());
        model.add_symbol_combination(&reached)?;

        // MTZ 子回路消除符号 / MTZ subtour elimination symbol
        let mtz = flat_map2_indexed("mtz", &data.cities, &data.cities, |i, _city_i, j, _city_j| {
            let monomials: Vec<_> = vec![
                ospf_rust_core::symbol::flatten::LinearMonomial::new(1.0, u_idx[&[i]]),
                ospf_rust_core::symbol::flatten::LinearMonomial::new(-1.0, u_idx[&[j]]),
                ospf_rust_core::symbol::flatten::LinearMonomial::new(n, x_idx[&[i, j]]),
            ];
            ospf_rust_core::symbol::flatten::Linear::new(monomials, 0.0)
        }, |_, city_i, _, city_j| format!("{}_{}", city_i.name, city_j.name));
        model.add_symbol_combination(&mtz)?;

        Ok(TspModel {
            x_vars,
            x_idx,
            u_vars,
            u_idx,
            distance,
            depart,
            reached,
            mtz,
        })
    }

    /// 添加约束和目标 / Add constraints and objective
    fn add_constraints(
        &self,
        model: &mut MetaModel<f64>,
        data: &TspData,
    ) -> Result<(), Box<dyn Error>> {
        let n = data.cities.len() as f64;
        let city_count = data.cities.len();

        // 目标: 最小化距离 / Objective: minimize distance
        let mut dist_coeffs = Vec::new();
        for i in 0..city_count {
            let poly = self.distance.symbol_polynomial(i);
            for m in poly.monomials() {
                dist_coeffs.push((m.var_index(), *m.coefficient()));
            }
        }
        let distance_input = LinearObjectiveInput::minimize("distance")
            .terms(dist_coeffs.into_iter());
        model.set_linear_objective_input(distance_input);

        // 每城市出发约束 / Depart constraint per city
        for i in 0..city_count {
            let coeffs = extract_coeffs(&self.depart[i]);
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 1.0, &format!("depart_{}", i))?;
        }

        // 每城市到达约束 / Arrive constraint per city
        for j in 0..city_count {
            let coeffs = extract_coeffs(&self.reached[j]);
            model.add_linear_constraint(&coeffs, ConstraintRelation::Equal, 1.0, &format!("arrive_{}", j))?;
        }

        // MTZ 子回路消除约束 / MTZ subtour elimination constraints
        for i in 0..city_count {
            if i == data.begin_idx {
                continue;
            }
            for j in 0..city_count {
                if j == data.begin_idx || i == j {
                    continue;
                }
                let coeffs = extract_coeffs(&self.mtz[&[i, j]]);
                model.add_linear_constraint(
                    &coeffs,
                    ConstraintRelation::LessEqual,
                    n - 1.0,
                    &format!("mtz_{}_{}", i, j),
                )?;
            }
        }

        Ok(())
    }
}

/// Demo10 主函数：旅行商问题（TSP） / Demo10 main function: Traveling Salesman Problem (TSP)
pub fn run() -> Result<(), Box<dyn Error>> {
    let data = TspData::sample();
    let city_count = data.cities.len();

    let mut model = MetaModel::<f64>::new("demo10");
    let tsp = TspModel::register(&mut model, &data)?;

    tsp.add_constraints(&mut model, &data)?;

    let output = solve_typed(model)?;
    let solution = output.solution;

    println!("=== Demo10 ===");
    println!("status: {:?}", output.status);
    if let Some(obj) = output.objective_value {
        println!("distance: {:.2}", obj);
    }
    for i in 0..city_count {
        for j in 0..city_count {
            if read_solution_value(&solution, tsp.x_idx[&[i, j]]) > 0.5 {
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
