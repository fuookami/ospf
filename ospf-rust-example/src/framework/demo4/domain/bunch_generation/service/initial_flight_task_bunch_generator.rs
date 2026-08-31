use std::collections::HashMap;
use super::super::model::Graph;

/// 初始飞行任务束生成器 / Initial flight task bunch generator
/// 对齐 FSRA InitialFlightTaskBunchGenerator
pub struct InitialFlightTaskBunchGenerator;

impl InitialFlightTaskBunchGenerator {
    /// 生成初始束 / Generate initial bunches
    /// 对齐 FSRA InitialFlightTaskBunchGenerator.invoke
    pub fn generate(
        &self,
        aircraft_ids: &[String],
        graphs: &HashMap<String, Graph>,
        locked_tasks: &[String],
    ) -> Vec<Vec<String>> {
        let mut bunches = Vec::new();

        for aircraft_id in aircraft_ids {
            if let Some(graph) = graphs.get(aircraft_id) {
                // 为每架飞机生成初始束
                let bunch = self.generate_for_aircraft(aircraft_id, graph, locked_tasks);
                if !bunch.is_empty() {
                    bunches.push(bunch);
                }
            }
        }

        // 添加 locked task 覆盖
        for task_id in locked_tasks {
            if !bunches.iter().any(|b| b.contains(task_id)) {
                bunches.push(vec![task_id.clone()]);
            }
        }

        bunches
    }

    fn generate_for_aircraft(
        &self,
        _aircraft_id: &str,
        graph: &Graph,
        locked_tasks: &[String],
    ) -> Vec<String> {
        let mut bunch = Vec::new();

        // DFS 从 Root 到 End 找一条路径
        let mut stack = vec![(super::super::model::Node::Root, Vec::new())];
        while let Some((node, mut path)) = stack.pop() {
            if node.is_end() {
                if path.len() > bunch.len() {
                    bunch = path;
                }
                continue;
            }
            if let Some(task_id) = node.task_id() {
                path.push(task_id.to_string());
            }
            for edge in graph.get_edges(&node) {
                stack.push((edge.to, path.clone()));
            }
        }

        // 确保 locked tasks 被包含
        for task_id in locked_tasks {
            if !bunch.contains(task_id) {
                bunch.push(task_id.clone());
            }
        }

        bunch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::model::{Graph, Node};
    use time::macros::datetime;

    fn task_node(id: &str, index: u64) -> Node {
        Node::Task {
            task_id: id.to_string(),
            time: datetime!(2026-06-07 08:00 UTC),
            index,
        }
    }

    #[test]
    fn generates_bunch_for_each_aircraft() {
        let mut graphs = HashMap::new();

        let mut graph1 = Graph::new();
        let t1 = task_node("T1", 1);
        graph1.put_node(t1.clone());
        graph1.put_edge(Node::Root, t1.clone());
        graph1.put_edge(t1, Node::End);
        graphs.insert("AC1".to_string(), graph1);

        let generator = InitialFlightTaskBunchGenerator;
        let bunches = generator.generate(
            &["AC1".to_string()],
            &graphs,
            &[],
        );
        assert_eq!(bunches.len(), 1);
        assert!(bunches[0].contains(&"T1".to_string()));
    }

    #[test]
    fn locked_tasks_are_included() {
        let graphs = HashMap::new(); // no graphs
        let generator = InitialFlightTaskBunchGenerator;
        let bunches = generator.generate(
            &["AC1".to_string()],
            &graphs,
            &["LOCKED".to_string()],
        );
        assert!(bunches.iter().any(|b| b.contains(&"LOCKED".to_string())));
    }

    #[test]
    fn empty_graph_produces_empty_bunch() {
        let mut graphs = HashMap::new();
        graphs.insert("AC1".to_string(), Graph::new());

        let generator = InitialFlightTaskBunchGenerator;
        let bunches = generator.generate(
            &["AC1".to_string()],
            &graphs,
            &[],
        );
        // Empty graph has Root -> End path, but no tasks
        assert!(bunches.is_empty() || bunches[0].is_empty());
    }
}
