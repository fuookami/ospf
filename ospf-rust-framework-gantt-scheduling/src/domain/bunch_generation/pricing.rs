//! 束定价问题 / Bunch pricing problem
//!
//! 实现列生成定价问题的求解，基于 Label Setting 算法。
//! Implements column generation pricing problem solving via Label Setting algorithm.

use std::collections::{HashMap, HashSet};

use crate::domain::bunch_compilation::model::BunchEntry;
use crate::domain::bunch_generation::label::Label;
use crate::domain::bunch_generation::model::{Graph, Node};
use crate::domain::task::Cost;

/// 束定价问题 / Bunch pricing problem
///
/// 基于影子价格和 Graph/Label 算法生成新束。
/// Generates new bunches based on shadow prices and Graph/Label algorithm.
pub struct BunchPricingProblem {
    /// 定价图 / Pricing graph
    pub graph: Graph,
    /// 执行器 ID / Executor ID
    pub executor_id: String,
    /// 任务影子价格 / Task shadow prices (task_index -> price)
    pub task_shadow_prices: HashMap<usize, f64>,
}

impl BunchPricingProblem {
    /// 创建定价问题 / Create pricing problem
    pub fn new(executor_id: String, graph: Graph) -> Self {
        Self {
            graph,
            executor_id,
            task_shadow_prices: HashMap::new(),
        }
    }

    /// 设置影子价格 / Set shadow prices
    pub fn set_shadow_prices(&mut self, prices: HashMap<usize, f64>) {
        self.task_shadow_prices = prices;
    }

    /// 求解定价问题 / Solve pricing problem
    ///
    /// 返回 reduced cost < 0 的束列表。
    /// Returns list of bunches with negative reduced cost.
    pub fn solve(&self, bunch_index_offset: usize) -> Vec<BunchEntry> {
        let algorithm = LabelSettingAlgorithm::new(
            &self.graph,
            &self.task_shadow_prices,
        );

        let labels = algorithm.run();
        let mut bunches = Vec::new();

        for (i, label) in labels.into_iter().enumerate() {
            if label.is_better_bunch() {
                if let Some(task_indices) = label.generate_bunch_tasks() {
                    if !task_indices.is_empty() {
                        let reduced_cost = label.reduced_cost();
                        bunches.push(BunchEntry {
                            index: bunch_index_offset + i,
                            executor_id: self.executor_id.clone(),
                            task_indices,
                            cost: reduced_cost,
                            iteration: 0, // 由调用方设置
                        });
                    }
                }
            }
        }

        bunches
    }
}

/// Label Setting 算法 / Label Setting algorithm
///
/// 动态规划算法，用于在定价图中搜索 reduced cost < 0 的路径。
/// Dynamic programming algorithm for searching paths with negative reduced cost
/// in the pricing graph.
pub struct LabelSettingAlgorithm<'a> {
    /// 定价图 / Pricing graph
    graph: &'a Graph,
    /// 任务影子价格 / Task shadow prices
    shadow_prices: &'a HashMap<usize, f64>,
}

impl<'a> LabelSettingAlgorithm<'a> {
    /// 创建 Label Setting 算法 / Create Label Setting algorithm
    pub fn new(
        graph: &'a Graph,
        shadow_prices: &'a HashMap<usize, f64>,
    ) -> Self {
        Self { graph, shadow_prices }
    }

    /// 执行算法 / Run the algorithm
    ///
    /// 返回到达 End 节点的所有非被支配标签。
    /// Returns all non-dominated labels reaching the End node.
    pub fn run(&self) -> Vec<Label> {
        // 默认实现：广度优先搜索 + 支配剪枝
        // Default implementation: BFS with dominance pruning
        let mut current_labels = vec![Label::root()];
        let mut final_labels: Vec<Label> = Vec::new();
        let mut visited_sets: Vec<HashSet<usize>> = Vec::new();

        // 最大迭代次数防止无限循环
        let max_iterations = 100;
        for _ in 0..max_iterations {
            let mut next_labels = Vec::new();

            for label in &current_labels {
                // 获取当前节点的出边
                let current_node = match &label.node {
                    Some(node) => node.clone(),
                    None => continue,
                };

                let edges = self.graph.edges_from(&current_node);

                for edge in edges {
                    let target = &edge.to;

                    // 检查目标节点是否已在路径中（避免环）
                    if label.visited(target) {
                        continue;
                    }

                    // 扩展标签
                    let new_label = match target {
                        Node::Task(task_node) => {
                            let shadow_price = self.shadow_prices
                                .get(&task_node.task_index)
                                .copied()
                                .unwrap_or(0.0);
                            let cost = Cost::empty();
                            Label::task(
                                label.clone(),
                                target.clone(),
                                task_node.task_index,
                                cost,
                                shadow_price,
                            )
                        }
                        Node::End => {
                            Label::end(label.clone())
                        }
                        Node::Root => continue, // 不回到根节点
                    };

                    // 如果到达 End 节点，保存
                    if matches!(&new_label.node, Some(Node::End)) {
                        final_labels.push(new_label);
                    } else {
                        // 支配检查：移除被支配的标签
                        if let Some(new_tasks) = new_label.generate_bunch_tasks() {
                            let new_tasks_set: HashSet<usize> = new_tasks.into_iter().collect();

                            let is_dominated = visited_sets.iter().any(|vs| {
                                vs.iter().all(|t| new_tasks_set.contains(t)) && vs.len() >= new_tasks_set.len()
                            });

                            if !is_dominated {
                                visited_sets.push(new_tasks_set);
                                next_labels.push(new_label);
                            }
                        } else {
                            next_labels.push(new_label);
                        }
                    }
                }
            }

            if next_labels.is_empty() {
                break;
            }
            current_labels = next_labels;
        }

        // 过滤出 reduced cost < 0 的标签
        final_labels.into_iter().filter(|l| l.is_better_bunch()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::bunch_generation::model::{Node, TaskNode};

    #[test]
    fn test_bunch_pricing_problem_simple() {
        let mut graph = Graph::new();

        let task0 = Node::Task(TaskNode::new(0, 1, 0.0));
        let task1 = Node::Task(TaskNode::new(1, 2, 0.0));

        graph.add_node(task0.clone());
        graph.add_node(task1.clone());

        // Root -> Task0 -> End
        graph.add_edge(graph.root().clone(), task0.clone());
        graph.add_edge(task0, graph.end().clone());

        let mut pricing = BunchPricingProblem::new("exec_1".to_string(), graph);
        pricing.set_shadow_prices(HashMap::from([
            (0, 1.0),
        ]));

        let _bunches = pricing.solve(0);
        // 应生成包含 task 0 的束（reduced cost 取决于 cost 设置）
        // 空成本 + 正影子价格 → reduced_cost 可能 < 0
    }

    #[test]
    fn test_label_setting_algorithm_basic() {
        let graph = Graph::new();
        let shadow_prices = HashMap::new();

        let algorithm = LabelSettingAlgorithm::new(&graph, &shadow_prices);
        let labels = algorithm.run();

        // 空图应该没有有效标签
        assert!(labels.is_empty());
    }
}
