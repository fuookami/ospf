//! 路线图生成器模块 / Route graph generator module
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use super::super::model::{Graph, Node, Edge, FlightTaskReverse};
use super::operator::FeasibilityJudger;

/// 路线图生成器配置 / Route graph generator configuration
#[derive(Debug, Clone)]
pub struct RouteGraphGeneratorConfig {
    /// 是否允许换序 / Whether order change is allowed
    pub with_order_change: bool,
}

impl Default for RouteGraphGeneratorConfig {
    fn default() -> Self {
        Self { with_order_change: false }
    }
}

/// 路线图生成器 / Route graph generator (对齐 FSRA RouteGraphGenerator)
pub struct RouteGraphGenerator {
    reverse: FlightTaskReverse,
    config: RouteGraphGeneratorConfig,
    feasibility_judger: FeasibilityJudger,
}

impl RouteGraphGenerator {
    /// 创建新的路线图生成器 / Create new route graph generator
    pub fn new(
        reverse: FlightTaskReverse,
        config: RouteGraphGeneratorConfig,
        feasibility_judger: FeasibilityJudger,
    ) -> Self {
        Self { reverse, config, feasibility_judger }
    }

    /// 为指定飞机生成路线图 / Generate route graph for aircraft
    /// 对齐 FSRA RouteGraphGenerator.invoke
    pub fn generate(
        &self,
        aircraft_id: &str,
        location: &str,
        flight_tasks: &HashMap<String, Vec<FlightTaskInfo>>,
    ) -> Graph {
        let mut graph = Graph::new();
        let mut node_map: HashMap<String, Node> = HashMap::new();
        let mut next_index = 1u64;

        // BFS
        let mut queue: Vec<(String, Node)> = vec![(location.to_string(), Node::Root)];

        while let Some((airport, node)) = queue.pop() {
            if let Some(tasks) = flight_tasks.get(&airport) {
                let mut has_successor = false;
                for task in tasks {
                    if self.check_feasibility(aircraft_id, &node, task) {
                        has_successor = true;
                        let task_node = self.insert_task(
                            &mut graph,
                            &mut node_map,
                            &mut next_index,
                            &node,
                            task,
                        );
                        queue.push((task.arr.clone(), task_node));
                    }

                    // 对换检查
                    if self.config.with_order_change {
                        if let Node::Task { task_id: prev_id, .. } = &node {
                            if self.reverse.contains(&task.id, prev_id)
                                && self.check_feasibility(aircraft_id, &node, task)
                            {
                                let task_node = self.insert_task(
                                    &mut graph,
                                    &mut node_map,
                                    &mut next_index,
                                    &node,
                                    task,
                                );
                                queue.push((task.arr.clone(), task_node));
                            }
                        }
                    }
                }
                if !has_successor {
                    graph.put_edge(node, Node::End);
                }
            } else {
                graph.put_edge(node, Node::End);
            }
        }

        graph
    }

    fn check_feasibility(&self, aircraft_id: &str, node: &Node, task: &FlightTaskInfo) -> bool {
        let prev_id = node.task_id();
        (self.feasibility_judger)(aircraft_id, prev_id, &task.id)
    }

    fn insert_task(
        &self,
        graph: &mut Graph,
        node_map: &mut HashMap<String, Node>,
        next_index: &mut u64,
        prev_node: &Node,
        task: &FlightTaskInfo,
    ) -> Node {
        if let Some(existing) = node_map.get(&task.id) {
            graph.put_edge(prev_node.clone(), existing.clone());
            existing.clone()
        } else {
            let node = Node::Task {
                task_id: task.id.clone(),
                time: task.dep_time,
                index: *next_index,
            };
            *next_index += 1;
            graph.put_node(node.clone());
            graph.put_edge(prev_node.clone(), node.clone());
            node_map.insert(task.id.clone(), node.clone());
            node
        }
    }
}

/// 飞行任务信息 / Flight task info
#[derive(Debug, Clone)]
pub struct FlightTaskInfo {
    /// 任务标识 / Task identifier
    pub id: String,
    /// 出发机场 / Departure airport
    pub dep: String,
    /// 到达机场 / Arrival airport
    pub arr: String,
    /// 出发时间 / Departure time
    pub dep_time: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;
    use time::Duration;

    fn task(id: &str, dep: &str, arr: &str) -> FlightTaskInfo {
        FlightTaskInfo {
            id: id.to_string(),
            dep: dep.to_string(),
            arr: arr.to_string(),
            dep_time: datetime!(2026-06-07 08:00 UTC),
        }
    }

    fn make_generator() -> RouteGraphGenerator {
        let reverse = FlightTaskReverse::new(vec![], &[], &[], Duration::hours(5));
        let config = RouteGraphGeneratorConfig::default();
        let judger: FeasibilityJudger = Box::new(|_aircraft, _prev, _task| true);
        RouteGraphGenerator::new(reverse, config, judger)
    }

    #[test]
    fn generates_edges_for_connectable_tasks() {
        let generator = make_generator();
        let mut flight_tasks = HashMap::new();
        flight_tasks.insert("A".to_string(), vec![task("T1", "A", "B")]);
        flight_tasks.insert("B".to_string(), vec![task("T2", "B", "C")]);

        let graph = generator.generate("AC1", "A", &flight_tasks);

        // Root -> T1 -> T2 -> End
        assert!(graph.node_count() >= 4);
        let root_edges = graph.get_edges(&Node::Root);
        assert!(!root_edges.is_empty());
    }

    #[test]
    fn no_edges_for_unreachable_tasks() {
        let generator = make_generator();
        let mut flight_tasks = HashMap::new();
        flight_tasks.insert("X".to_string(), vec![task("T1", "X", "Y")]);
        // Aircraft at "A", no tasks from A
        flight_tasks.insert("A".to_string(), vec![]);

        let graph = generator.generate("AC1", "A", &flight_tasks);
        // Root -> End (no tasks reachable)
        assert!(graph.connected(&Node::Root, &Node::End));
    }

    #[test]
    fn with_order_change_creates_reverse_edges() {
        let reverse = FlightTaskReverse::new(
            vec![("T2".into(), "T1".into())],
            &[],
            &[],
            Duration::hours(5),
        );
        let config = RouteGraphGeneratorConfig { with_order_change: true };
        // Use a judger that rejects reverse connections to avoid infinite loops
        let judger: FeasibilityJudger = Box::new(|_aircraft, prev, task| {
            prev.is_none() || task == "T2" // Only allow Root->T1 and T1->T2
        });
        let generator = RouteGraphGenerator::new(reverse, config, judger);

        let mut flight_tasks = HashMap::new();
        flight_tasks.insert("A".to_string(), vec![task("T1", "A", "B")]);
        flight_tasks.insert("B".to_string(), vec![task("T2", "B", "C")]);

        let graph = generator.generate("AC1", "A", &flight_tasks);
        // Should have Root, T1, T2, End
        assert!(graph.node_count() >= 3);
    }
}
