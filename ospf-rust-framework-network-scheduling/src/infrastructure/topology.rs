//! 与领域载荷无关的拓扑算法 / Topology algorithms independent of domain payloads.

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::marker::PhantomData;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArc, NetworkArcId, NetworkGraphAccess, NetworkNodeId, validate_linear_units};

/// 目标相关最短路结果 / Target-relevant shortest-path outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum ShortestPathResult<V> {
    /// 有限最短路及有序稳定弧身份 / Finite shortest path and ordered stable arc identities.
    Found {
        /// 路径总权重 / Total path weight.
        cost: Quantity<V, Unit>,
        /// 保留平行服务身份的弧序列 / Arc sequence preserving parallel service identities.
        arcs: Vec<NetworkArcId>,
    },
    /// 无法到达目标 / Target is unreachable.
    Unreachable,
    /// 起点可达且能到达目标的负环 / Negative cycle reachable from the source and reaching the target.
    NegativeCycle,
}

/// 面向只读图访问器的拓扑入口 / Topology facade over a read-only graph accessor.
pub struct NetworkTopology<'a, N, A, G> {
    graph: &'a G,
    marker: PhantomData<fn() -> (N, A)>,
}

struct IndexedGraph<'a, A> {
    nodes: Vec<NetworkNodeId>,
    indices: HashMap<NetworkNodeId, usize>,
    arcs: Vec<&'a NetworkArc<A>>,
    endpoints: Vec<(usize, usize)>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
}

impl<A> IndexedGraph<'_, A> {
    fn index(&self, id: &NetworkNodeId) -> Result<usize> {
        self.indices.get(id).copied().ok_or_else(|| NetworkSchedulingError::validation(
            format!("节点不在当前视图中：{id} / node is not in the current view: {id}")))
    }

    fn reachable(&self, start: usize, reverse: bool) -> Vec<bool> {
        let mut seen = vec![false; self.nodes.len()];
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(node) = queue.pop_front() {
            let adjacent = if reverse { &self.incoming[node] } else { &self.outgoing[node] };
            for &arc in adjacent {
                let next = if reverse { self.endpoints[arc].0 } else { self.endpoints[arc].1 };
                if !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
        seen
    }

    fn trace(&self, start: usize, target: usize, parent: &[Option<usize>]) -> Result<Vec<NetworkArcId>> {
        let mut current = target;
        let mut path = Vec::new();
        while current != start {
            let edge = parent[current].ok_or_else(|| NetworkSchedulingError::contract(
                "路径前驱缺失 / path predecessor is missing"))?;
            path.push(self.arcs[edge].id.clone());
            current = self.endpoints[edge].0;
            if path.len() >= self.nodes.len() {
                return Err(NetworkSchedulingError::contract("路径前驱成环 / cyclic path predecessors"));
            }
        }
        path.reverse();
        Ok(path)
    }
}

#[derive(PartialEq)]
struct Distance<V>(V, usize);

impl<V: PartialOrd> Eq for Distance<V> {}

impl<V: PartialOrd> PartialOrd for Distance<V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl<V: PartialOrd> Ord for Distance<V> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal).then_with(|| other.1.cmp(&self.1))
    }
}

// 在原数值域内检查有限性，避免把任意精度权重限制到 f64 表示范围。 / Check finiteness natively without restricting arbitrary-precision weights to the f64 range.
fn finite_weight<V: SolveValue + UnitConversionValue>(value: V, zero: &V) -> Result<V> {
    if (value.clone() - value.clone()).partial_cmp(zero) == Some(Ordering::Equal) {
        Ok(value)
    } else {
        Err(NetworkSchedulingError::validation("最短路权重及累计值必须有限 / shortest-path weights and sums must be finite"))
    }
}

impl<'a, N, A, G> NetworkTopology<'a, N, A, G>
where
    G: NetworkGraphAccess<N, A>,
{
    /// 使用当前图或过滤/反向视图创建入口 / Use the current graph or filtered/reversed view.
    pub fn new(graph: &'a G) -> Self { Self { graph, marker: PhantomData } }

    fn indexed(&self) -> Result<IndexedGraph<'_, A>> {
        let nodes: Vec<_> = self.graph.nodes().map(|node| node.id.clone()).collect();
        let indices: HashMap<_, _> = nodes.iter().cloned().enumerate().map(|(i, id)| (id, i)).collect();
        if indices.len() != nodes.len() {
            return Err(NetworkSchedulingError::structure("节点身份重复 / duplicate node identities"));
        }
        let arcs: Vec<_> = self.graph.arcs().collect();
        let mut identities = BTreeSet::new();
        let mut endpoints = Vec::with_capacity(arcs.len());
        let mut outgoing = vec![Vec::new(); nodes.len()];
        let mut incoming = vec![Vec::new(); nodes.len()];
        for (index, arc) in arcs.iter().enumerate() {
            if !identities.insert(&arc.id) {
                return Err(NetworkSchedulingError::structure("弧身份重复 / duplicate arc identities"));
            }
            let from = indices.get(self.graph.from(arc)).copied();
            let to = indices.get(self.graph.to(arc)).copied();
            let (Some(from), Some(to)) = (from, to) else {
                return Err(NetworkSchedulingError::structure("视图弧端点缺失 / missing view arc endpoint"));
            };
            endpoints.push((from, to));
            outgoing[from].push(index);
            incoming[to].push(index);
        }
        Ok(IndexedGraph { nodes, indices, arcs, endpoints, outgoing, incoming })
    }

    /// 从起点可达的所有节点（包括起点） / All reachable nodes including the source.
    pub fn reachable(&self, start: &NetworkNodeId) -> Result<BTreeSet<NetworkNodeId>> {
        let graph = self.indexed()?;
        let seen = graph.reachable(graph.index(start)?, false);
        Ok(graph.nodes.into_iter().zip(seen).filter_map(|(id, seen)| seen.then_some(id)).collect())
    }

    /// 最少弧数路径见证，保留稳定弧身份 / Minimum-hop path witness with stable arc identities.
    pub fn path(&self, start: &NetworkNodeId, target: &NetworkNodeId) -> Result<Option<Vec<NetworkArcId>>> {
        let graph = self.indexed()?;
        let start = graph.index(start)?;
        let target = graph.index(target)?;
        let mut seen = vec![false; graph.nodes.len()];
        let mut parent = vec![None; graph.nodes.len()];
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        while let Some(node) = queue.pop_front() {
            if node == target { return graph.trace(start, target, &parent).map(Some); }
            for &arc in &graph.outgoing[node] {
                let next = graph.endpoints[arc].1;
                if !seen[next] {
                    seen[next] = true;
                    parent[next] = Some(arc);
                    queue.push_back(next);
                }
            }
        }
        Ok(None)
    }

    /// 返回 DAG 拓扑序；存在有向环时为空 / Return DAG order, or none for directed cycles.
    pub fn topological_order(&self) -> Result<Option<Vec<NetworkNodeId>>> {
        let graph = self.indexed()?;
        let mut degrees: Vec<_> = graph.incoming.iter().map(Vec::len).collect();
        let mut queue: VecDeque<_> = degrees.iter().enumerate().filter_map(|(i, &d)| (d == 0).then_some(i)).collect();
        let mut order = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(graph.nodes[node].clone());
            for &arc in &graph.outgoing[node] {
                let next = graph.endpoints[arc].1;
                degrees[next] -= 1;
                if degrees[next] == 0 { queue.push_back(next); }
            }
        }
        Ok((order.len() == graph.nodes.len()).then_some(order))
    }

    /// 非递归 Kosaraju 强连通分量 / Non-recursive Kosaraju strongly connected components.
    pub fn strongly_connected_components(&self) -> Result<Vec<Vec<NetworkNodeId>>> {
        let graph = self.indexed()?;
        let mut seen = vec![false; graph.nodes.len()];
        let mut order = Vec::new();
        for start in 0..graph.nodes.len() {
            if seen[start] { continue; }
            seen[start] = true;
            let mut stack = vec![(start, 0)];
            while let Some((node, offset)) = stack.last_mut() {
                if let Some(&arc) = graph.outgoing[*node].get(*offset) {
                    *offset += 1;
                    let next = graph.endpoints[arc].1;
                    if !seen[next] {
                        seen[next] = true;
                        stack.push((next, 0));
                    }
                } else {
                    order.push(*node);
                    stack.pop();
                }
            }
        }
        seen.fill(false);
        let mut components = Vec::new();
        for start in order.into_iter().rev() {
            if seen[start] { continue; }
            let mut stack = vec![start];
            let mut component = Vec::new();
            seen[start] = true;
            while let Some(node) = stack.pop() {
                component.push(graph.nodes[node].clone());
                for &arc in &graph.incoming[node] {
                    let next = graph.endpoints[arc].0;
                    if !seen[next] { seen[next] = true; stack.push(next); }
                }
            }
            components.push(component);
        }
        Ok(components)
    }

    /// 在指定线性单位和原数值域内求最短路；权重、比较及累计不经 f64。 / Find a shortest path in the requested linear unit and native value domain, without f64 weights, comparisons or sums.
    ///
    /// 目标费用单位与每条弧的权重单位须兼容；有限实数须具有全序比较和通常的加减语义。 / The target cost unit must be compatible with each arc weight; finite real values must have total comparison and ordinary addition/subtraction semantics.
    pub fn shortest_path<V, F>(
        &self,
        start: &NetworkNodeId,
        target: &NetworkNodeId,
        unit: &Unit,
        cost: F,
    ) -> Result<ShortestPathResult<V>>
    where
        V: SolveValue + UnitConversionValue,
        F: Fn(&NetworkArc<A>) -> Result<Quantity<V, Unit>>,
    {
        validate_linear_units([unit])?;
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::Strict)
            .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
        let zero = finite_weight(zero.clone(), &zero)?;
        let graph = self.indexed()?;
        let start = graph.index(start)?;
        let target = graph.index(target)?;
        let weights: Vec<_> = graph.arcs.iter().map(|arc| {
            let weight = cost(arc)?;
            validate_linear_units([&weight.unit])?;
            finite_weight(weight.value.clone(), &zero)?;
            let normalized = weight.to_unit(unit)
                .map_err(|error| NetworkSchedulingError::Conversion { message: error.to_string() })?;
            finite_weight(normalized.value, &zero)
        }).collect::<Result<_>>()?;
        let reachable = graph.reachable(start, false);
        if !reachable[target] { return Ok(ShortestPathResult::Unreachable); }
        let reaches_target = graph.reachable(target, true);
        let relevant: Vec<_> = reachable.iter().zip(reaches_target).map(|(a, b)| *a && b).collect();
        let mut distances = vec![None; graph.nodes.len()];
        let mut parent = vec![None; graph.nodes.len()];
        distances[start] = Some(zero.clone());
        if weights.iter().all(|weight| weight >= &zero) {
            let mut queue = BinaryHeap::from([Distance(zero.clone(), start)]);
            while let Some(Distance(distance, node)) = queue.pop() {
                if distances[node].as_ref().is_some_and(|current| &distance > current) { continue; }
                if node == target { break; }
                for &arc in &graph.outgoing[node] {
                    let next = graph.endpoints[arc].1;
                    if !relevant[next] { continue; }
                    let candidate = finite_weight(distance.clone() + weights[arc].clone(), &zero)?;
                    if distances[next].as_ref().is_none_or(|current| &candidate < current) {
                        distances[next] = Some(candidate.clone());
                        parent[next] = Some(arc);
                        queue.push(Distance(candidate, next));
                    }
                }
            }
        } else {
            for iteration in 0..graph.nodes.len() {
                let mut changed = false;
                for (arc, &(from, to)) in graph.endpoints.iter().enumerate() {
                    if !relevant[from] || !relevant[to] { continue; }
                    let Some(distance) = &distances[from] else { continue; };
                    let candidate = finite_weight(distance.clone() + weights[arc].clone(), &zero)?;
                    if distances[to].as_ref().is_none_or(|current| &candidate < current) {
                        if iteration + 1 == graph.nodes.len() { return Ok(ShortestPathResult::NegativeCycle); }
                        distances[to] = Some(candidate);
                        parent[to] = Some(arc);
                        changed = true;
                    }
                }
                if !changed { break; }
            }
        }
        let cost = distances[target].clone().ok_or_else(|| NetworkSchedulingError::contract("可达终点缺少距离 / reachable target lacks a distance"))?;
        Ok(ShortestPathResult::Found { cost: Quantity::new(cost, unit.clone()), arcs: graph.trace(start, target, &parent)? })
    }
}

#[cfg(test)]
#[allow(clippy::borrow_interior_mutable_const)]
mod tests {
    use super::*;
    use ospf_rust_quantities::unit::{CTUnit, Celsius, Kelvin, Kilometer, Meter, NoneUnit};
    use crate::infrastructure::{NetworkGraph, NetworkNode};

    fn weight(arc: &NetworkArc<f64>) -> Result<Quantity<f64, Unit>> {
        Ok(Quantity::new(arc.payload, NoneUnit::INSTANT.clone()))
    }

    fn graph(arcs: &[(&str, &str, &str, f64)]) -> NetworkGraph<(), f64> {
        NetworkGraph::new(["s", "a", "t", "x"].map(|id| NetworkNode::new(id, ())).to_vec(),
            arcs.iter().map(|&(id, from, to, cost)| NetworkArc::new(id, from, to, cost)).collect()).unwrap()
    }

    #[test]
    fn parallel_services_views_and_relevant_negative_cycles() {
        let graph = graph(&[("slow", "s", "a", 5.0), ("fast", "s", "a", 1.0),
            ("end", "a", "t", 2.0), ("dead", "s", "x", 1.0), ("loop", "x", "x", -1.0)]);
        let topology = NetworkTopology::new(&graph);
        assert_eq!(topology.shortest_path(&"s".into(), &"t".into(), &NoneUnit::INSTANT, weight).unwrap(),
            ShortestPathResult::Found { cost: Quantity::new(3.0, NoneUnit::INSTANT.clone()), arcs: vec!["fast".into(), "end".into()] });
        assert_eq!(topology.shortest_path(&"s".into(), &"x".into(), &NoneUnit::INSTANT, weight).unwrap(), ShortestPathResult::NegativeCycle);
        let view = graph.reverse().filtered_nodes(|node| node.id.as_str() != "x");
        assert_eq!(view.from(view.arc(&"end".into()).unwrap()), &NetworkNodeId::from("t"));
        assert_eq!(view.arc(&"end".into()).unwrap().from, NetworkNodeId::from("a"));
        assert_eq!(NetworkTopology::new(&view).path(&"t".into(), &"s".into()).unwrap().unwrap().len(), 2);
        assert!(NetworkTopology::new(&view).topological_order().unwrap().is_some());
        assert_eq!(topology.strongly_connected_components().unwrap().len(), 4);
    }

    #[test]
    fn dijkstra_negative_edges_and_zero_length_paths() {
        let graph = graph(&[("a", "s", "t", 8.0), ("b", "s", "a", 2.0), ("c", "a", "t", 1.0)]);
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(), &NoneUnit::INSTANT, weight).unwrap(),
            ShortestPathResult::Found { cost: Quantity::new(3.0, NoneUnit::INSTANT.clone()), arcs: vec!["b".into(), "c".into()] });
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"s".into(), &NoneUnit::INSTANT, weight).unwrap(),
            ShortestPathResult::Found { cost: Quantity::new(0.0, NoneUnit::INSTANT.clone()), arcs: vec![] });
        assert!(NetworkTopology::new(&graph).path(&"x".into(), &"s".into()).unwrap().is_none());
        assert!(NetworkTopology::new(&graph).reachable(&"unknown".into()).is_err());
        let filtered = graph.filtered(|arc| arc.id.as_str() != "a");
        assert!(filtered.arc(&"a".into()).is_none());
    }

    #[test]
    fn physical_weights_normalize_units_and_preserve_reverse_view_endpoints() {
        let graph = NetworkGraph::new(["s", "a", "t"].map(|id| NetworkNode::new(id, ())).to_vec(), vec![
            NetworkArc::new("sa", "s", "a", Quantity::new(0.001, Kilometer::INSTANT.clone())),
            NetworkArc::new("at", "a", "t", Quantity::new(2.0, Meter::INSTANT.clone())),
            NetworkArc::new("st", "s", "t", Quantity::new(4.0, Meter::INSTANT.clone())),
        ]).unwrap();
        let expected = ShortestPathResult::Found { cost: Quantity::new(3.0, Meter::INSTANT.clone()),
            arcs: vec!["sa".into(), "at".into()] };
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(),
            &Meter::INSTANT, |arc| Ok(arc.payload.clone())).unwrap(), expected);
        let reversed = graph.reverse();
        assert_eq!(NetworkTopology::new(&reversed).shortest_path(&"t".into(), &"s".into(),
            &Meter::INSTANT, |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(3.0, Meter::INSTANT.clone()), arcs: vec!["at".into(), "sa".into()],
            });
        assert!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(),
            &Kelvin::INSTANT, |arc| Ok(arc.payload.clone())).is_err());
    }

    #[test]
    fn invalid_affine_nonfinite_and_overflowing_weights_are_structured_errors() {
        let graph = graph(&[("sa", "s", "a", f64::MAX), ("at", "a", "t", f64::MAX)]);
        let topology = NetworkTopology::new(&graph);
        assert!(topology.shortest_path(&"s".into(), &"t".into(), &NoneUnit::INSTANT, weight).is_err());
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(topology.shortest_path(&"s".into(), &"t".into(), &NoneUnit::INSTANT,
                |_| Ok(Quantity::new(invalid, NoneUnit::INSTANT.clone()))).is_err());
        }
        assert!(topology.shortest_path(&"s".into(), &"t".into(), &Celsius::INSTANT,
            |_| Ok(Quantity::new(1.0, Kelvin::INSTANT.clone()))).is_err());
        assert!(topology.shortest_path(&"s".into(), &"t".into(), &Kelvin::INSTANT,
            |_| Ok(Quantity::new(1.0, Celsius::INSTANT.clone()))).is_err());
        assert!(topology.shortest_path(&"s".into(), &"t".into(), &Kelvin::INSTANT,
            |_| Ok(Quantity::new(1.0, Kelvin::INSTANT.clone()))).is_ok());
        assert!(topology.shortest_path(&"s".into(), &"t".into(), &Meter::INSTANT,
            |_| Ok(Quantity::new(f64::MAX, Kilometer::INSTANT.clone()))).is_err());
        assert!(topology.shortest_path::<f64, _>(&"s".into(), &"t".into(), &NoneUnit::INSTANT,
            |_| Err(NetworkSchedulingError::validation("测试回调失败 / test callback failure"))).is_err());
    }

    #[cfg(any(feature = "big-decimal", feature = "big-rational"))]
    fn exact_weights<V>(value: impl Fn(&str) -> V)
    where
        V: SolveValue + UnitConversionValue,
    {
        let unit = Meter::INSTANT.clone();
        let graph = NetworkGraph::new(["s", "a", "t", "x"].map(|id| NetworkNode::new(id, ())).to_vec(), vec![
            NetworkArc::new("expensive-first", "s", "t", Quantity::new(value("9007199254740993"), unit.clone())),
            NetworkArc::new("cheaper-second", "s", "t", Quantity::new(value("9007199254740992"), unit.clone())),
            NetworkArc::new("dead", "s", "x", Quantity::new(value("0"), unit.clone())),
            NetworkArc::new("negative-cycle", "x", "x", Quantity::new(value("-1"), unit.clone())),
        ]).unwrap();
        let topology = NetworkTopology::new(&graph);
        assert_eq!(topology.shortest_path(&"s".into(), &"t".into(), &unit, |arc| Ok(arc.payload.clone())).unwrap(),
            ShortestPathResult::Found { cost: Quantity::new(value("9007199254740992"), unit.clone()), arcs: vec!["cheaper-second".into()] });
        assert_eq!(topology.shortest_path(&"s".into(), &"x".into(), &unit, |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::NegativeCycle);
        let nonnegative = graph.filtered(|arc| arc.id.as_str() != "negative-cycle");
        assert_eq!(NetworkTopology::new(&nonnegative).shortest_path(&"s".into(), &"t".into(), &unit,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(value("9007199254740992"), unit), arcs: vec!["cheaper-second".into()],
            });
    }

    #[cfg(any(feature = "big-decimal", feature = "big-rational"))]
    fn sub_resolution_weights<V>(one: V, epsilon: V)
    where
        V: SolveValue + UnitConversionValue,
    {
        let half = one.clone() / (one.clone() + one.clone());
        let graph = NetworkGraph::new(["s", "a", "t"].map(|id| NetworkNode::new(id, ())).to_vec(), vec![
            NetworkArc::new("expensive-first", "s", "t", Quantity::new(one.clone() + epsilon.clone(), Meter::INSTANT.clone())),
            NetworkArc::new("cheaper-second", "s", "t", Quantity::new(one.clone(), Meter::INSTANT.clone())),
            NetworkArc::new("sa", "s", "a", Quantity::new(half.clone(), Meter::INSTANT.clone())),
            NetworkArc::new("at", "a", "t", Quantity::new(half + epsilon.clone(), Meter::INSTANT.clone())),
        ]).unwrap();
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(), &Meter::INSTANT,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(one.clone(), Meter::INSTANT.clone()), arcs: vec!["cheaper-second".into()],
            });
        let two = one.clone() + one.clone();
        let thousand = V::from_f64_with_policy(1000.0, SolveValueConversionPolicy::Strict).unwrap();
        let with_negative = NetworkGraph::new(["s", "a", "t"].map(|id| NetworkNode::new(id, ())).to_vec(), vec![
            NetworkArc::new("sa", "s", "a", Quantity::new(two.clone(), Kilometer::INSTANT.clone())),
            NetworkArc::new("at", "a", "t", Quantity::new(one.clone() - two * thousand + epsilon.clone(), Meter::INSTANT.clone())),
            NetworkArc::new("direct", "s", "t", Quantity::new(one.clone(), Meter::INSTANT.clone())),
        ]).unwrap();
        assert_eq!(NetworkTopology::new(&with_negative).shortest_path(&"s".into(), &"t".into(), &Meter::INSTANT,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(one.clone(), Meter::INSTANT.clone()), arcs: vec!["direct".into()],
            });
        let indirect = with_negative.filtered(|arc| arc.id.as_str() != "direct");
        assert_eq!(NetworkTopology::new(&indirect).shortest_path(&"s".into(), &"t".into(), &Meter::INSTANT,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(one + epsilon, Meter::INSTANT.clone()), arcs: vec!["sa".into(), "at".into()],
            });
    }

    #[cfg(feature = "big-decimal")]
    #[test]
    fn decimal_shortest_paths_keep_native_precision_and_values_outside_f64_range() {
        use bigdecimal::BigDecimal;
        exact_weights(|value| value.parse::<BigDecimal>().unwrap());
        sub_resolution_weights(BigDecimal::from(1), "1e-100".parse::<BigDecimal>().unwrap());
        let huge = "1e1000".parse::<BigDecimal>().unwrap();
        let graph = NetworkGraph::new(["s", "t"].map(|id| NetworkNode::new(id, ())).to_vec(),
            vec![NetworkArc::new("huge", "s", "t", Quantity::new(huge.clone(), Meter::INSTANT.clone()))]).unwrap();
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(), &Meter::INSTANT,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new(huge, Meter::INSTANT.clone()), arcs: vec!["huge".into()],
            });
        assert_eq!(NetworkTopology::new(&graph).shortest_path(&"s".into(), &"t".into(), &Kilometer::INSTANT,
            |arc| Ok(arc.payload.clone())).unwrap(), ShortestPathResult::Found {
                cost: Quantity::new("1e997".parse::<BigDecimal>().unwrap(), Kilometer::INSTANT.clone()), arcs: vec!["huge".into()],
            });
    }

    #[cfg(feature = "big-rational")]
    #[test]
    fn rational_shortest_paths_keep_native_precision() {
        exact_weights(|value| num_rational::BigRational::from_integer(value.parse().unwrap()));
        sub_resolution_weights(num_rational::BigRational::from_integer(1.into()),
            num_rational::BigRational::new(1.into(), 10.into()).pow(100));
    }
}
