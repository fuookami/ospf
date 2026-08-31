//! 标签模型 / Label model
//!
//! 定义列生成定价问题中的标签（部分路径），用于动态规划搜索。
//! Defines labels (partial paths) for column generation pricing via dynamic programming.

use std::collections::HashSet;

use crate::domain::task::Cost;

use super::model::Node;

// ============================================================================
// 标签 / Label
// ============================================================================

/// 标签 / Label
///
/// 表示定价图中的部分路径，包含累积成本、影子价格和路径追踪。
/// Represents a partial path in the pricing graph with accumulated cost, shadow price, and trace.
#[derive(Debug, Clone)]
pub struct Label {
    /// 累积成本 / Accumulated cost
    pub cost: Cost<f64>,
    /// 影子价格 / Shadow price from LP relaxation
    pub shadow_price: f64,
    /// 前驱标签 / Previous label (linked list)
    pub prev_label: Option<Box<Label>>,
    /// 当前节点 / Current node
    pub node: Option<Node>,
    /// 当前任务索引 / Current task index
    pub task_index: Option<usize>,
}

impl Label {
    /// 创建根标签 / Create root label
    ///
    /// 根标签位于根节点，无前驱，成本为零。
    /// Root label is at the root node with no predecessor and zero cost.
    pub fn root() -> Self {
        Self {
            cost: Cost::empty(),
            shadow_price: 0.0,
            prev_label: None,
            node: Some(Node::Root),
            task_index: None,
        }
    }

    /// 创建任务标签 / Create task label
    ///
    /// 从前驱标签扩展到任务节点，更新成本和影子价格。
    /// Extends from a predecessor label to a task node, updating cost and shadow price.
    pub fn task(
        prev: Label,
        node: Node,
        task_index: usize,
        cost: Cost<f64>,
        shadow_price: f64,
    ) -> Self {
        Self {
            cost,
            shadow_price,
            prev_label: Some(Box::new(prev)),
            node: Some(node),
            task_index: Some(task_index),
        }
    }

    /// 创建终止标签 / Create end label
    ///
    /// 到达终止节点的标签，可生成任务束。
    /// Label reaching the end node; can generate a bunch.
    pub fn end(prev: Label) -> Self {
        Self {
            cost: prev.cost.clone(),
            shadow_price: prev.shadow_price,
            prev_label: Some(Box::new(prev)),
            node: Some(Node::End),
            task_index: None,
        }
    }

    /// 缩减成本 / Reduced cost
    ///
    /// reduced_cost = cost_sum - shadow_price。
    /// 当 reduced_cost < 0 时，列为有吸引力的列。
    ///
    /// reduced_cost = cost_sum - shadow_price.
    /// When reduced_cost < 0, the column is attractive.
    pub fn reduced_cost(&self) -> f64 {
        self.cost.solver_cost(0.0) - self.shadow_price
    }

    /// 是否为更好的束（有吸引力的列）/ Whether this is a better bunch (attractive column)
    pub fn is_better_bunch(&self) -> bool {
        self.reduced_cost() < 0.0
    }

    /// 是否已访问指定节点 / Whether the specified node has been visited
    pub fn visited(&self, node: &Node) -> bool {
        self.trace_node_indices().contains(&node.index())
    }

    /// 获取路径中的任务索引列表 / Get task indices in the path
    pub fn trace_task_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        let mut current = Some(self);
        while let Some(label) = current {
            if let Some(ti) = label.task_index {
                indices.push(ti);
            }
            current = label.prev_label.as_ref().map(|b| b.as_ref());
        }
        indices.reverse();
        indices
    }

    /// 获取路径中的节点索引集合 / Get node indices in the path
    fn trace_node_indices(&self) -> HashSet<super::model::NodeIndex> {
        let mut indices = HashSet::new();
        let mut current = Some(self);
        while let Some(label) = current {
            if let Some(ref node) = label.node {
                indices.insert(node.index());
            }
            current = label.prev_label.as_ref().map(|b| b.as_ref());
        }
        indices
    }

    /// 从标签路径生成任务索引列表（仅终止标签有效）/ Generate task index list from label path (only valid for end labels)
    pub fn generate_bunch_tasks(&self) -> Option<Vec<usize>> {
        // 只有终止标签可以生成束
        match &self.node {
            Some(Node::End) => {}
            _ => return None,
        }

        // 回溯到根节点，收集任务
        let mut tasks = Vec::new();
        let mut current = Some(self);
        while let Some(label) = current {
            match &label.node {
                Some(Node::Root) => break,
                Some(Node::Task(_)) => {
                    if let Some(ti) = label.task_index {
                        tasks.push(ti);
                    }
                }
                Some(Node::End) => {} // 跳过终止节点
                None => {
                    if let Some(ti) = label.task_index {
                        tasks.push(ti);
                    }
                }
            }
            current = label.prev_label.as_ref().map(|b| b.as_ref());
        }

        tasks.reverse();
        Some(tasks)
    }
}

// ============================================================================
// 标签支配 / Label Dominance
// ============================================================================

/// 标签支配检查 / Label dominance check
///
/// 如果 label_a 的成本不高于 label_b 且影子价格不低于 label_b，
/// 且 a 访问的任务是 b 的子集，则 a 支配 b。
///
/// If label_a's cost is no higher than label_b's and shadow price no lower,
/// and a's visited tasks are a subset of b's, then a dominates b.
pub fn label_dominates(a: &Label, b: &Label) -> bool {
    // 成本更小或相等
    if a.cost.solver_cost(0.0) > b.cost.solver_cost(0.0) + 1e-9 {
        return false;
    }
    // 影子价格更大或相等
    if a.shadow_price < b.shadow_price - 1e-9 {
        return false;
    }
    // a 访问的节点是 b 的子集
    let a_nodes = a.trace_node_indices();
    let b_nodes = b.trace_node_indices();
    a_nodes.is_subset(&b_nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::CostItem;

    fn test_cost(value: f64) -> Cost<f64> {
        Cost::new(vec![CostItem::with_quantity("cost", value)])
    }

    #[test]
    fn test_root_label() {
        let label = Label::root();
        assert!(!label.is_better_bunch());
        assert_eq!(label.cost.solver_cost(0.0), 0.0);
        assert!(label.task_index.is_none());
    }

    #[test]
    fn test_label_chain() {
        let root = Label::root();

        let task1 = Label::task(
            root,
            Node::Task(super::super::model::TaskNode::new(0, 1, 100.0)),
            0,
            test_cost(5.0),
            2.0,
        );

        let task2 = Label::task(
            task1,
            Node::Task(super::super::model::TaskNode::new(1, 2, 200.0)),
            1,
            test_cost(8.0),
            3.0,
        );

        let end = Label::end(task2);

        // 验证路径
        let tasks = end.generate_bunch_tasks().unwrap();
        assert_eq!(tasks, vec![0, 1]);

        // 验证缩减成本
        assert!((end.reduced_cost() - (8.0 - 3.0)).abs() < 1e-9);
    }

    #[test]
    fn test_label_visited() {
        let root = Label::root();
        let task_node = Node::Task(super::super::model::TaskNode::new(0, 1, 100.0));
        let task_label = Label::task(root, task_node.clone(), 0, test_cost(5.0), 0.0);

        assert!(task_label.visited(&Node::Root));
        assert!(task_label.visited(&task_node));
        assert!(!task_label.visited(&Node::End));
    }

    #[test]
    fn test_label_dominance() {
        let task_node = Node::Task(super::super::model::TaskNode::new(0, 1, 100.0));

        let label_a = Label::task(
            Label::root(),
            task_node.clone(),
            0,
            test_cost(5.0),
            3.0,
        );

        let label_b = Label::task(
            Label::root(),
            task_node.clone(),
            0,
            test_cost(8.0),
            2.0,
        );

        // a: cost=5, shadow=3; b: cost=8, shadow=2
        // a has lower cost and higher shadow price
        assert!(label_dominates(&label_a, &label_b));
        assert!(!label_dominates(&label_b, &label_a));
    }
}
