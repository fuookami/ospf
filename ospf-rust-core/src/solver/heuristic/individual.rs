//! 个体 / Individual

use crate::model::callback::Solution;

/// 带适应度的解 / Solution with fitness
///
/// 启发式搜索的最小信息单元：一个解及其目标值（适应度）。所有算子都以个体列表为输入输出。
/// The minimal information unit of heuristic search: a solution together with its objective value (fitness).
/// Every operator consumes and produces lists of individuals.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 解值类型 / Solution value type
#[derive(Debug, Clone)]
pub struct Individual<O, V> {
    /// 解 / Solution
    pub solution: Solution<V>,
    /// 适应度（目标值）/ Fitness (objective value)
    pub fitness: O,
}

impl<O, V> Individual<O, V> {
    /// 创建个体 / Create an individual
    pub fn new(solution: Solution<V>, fitness: O) -> Self {
        Self { solution, fitness }
    }

    /// 获取解引用 / Get the solution reference
    pub fn solution(&self) -> &Solution<V> {
        &self.solution
    }

    /// 获取适应度引用 / Get the fitness reference
    pub fn fitness(&self) -> &O {
        &self.fitness
    }

    /// 解值维度 / Solution dimensionality
    pub fn len(&self) -> usize {
        self.solution.len()
    }

    /// 解是否为空 / Whether the solution is empty
    pub fn is_empty(&self) -> bool {
        self.solution.is_empty()
    }
}
