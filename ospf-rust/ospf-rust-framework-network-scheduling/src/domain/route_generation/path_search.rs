//! 路线标签与共享路径内核的适配，不另建搜索循环。 / Route-label adapter to the shared path kernel, without another traversal loop.

use std::cmp::Ordering;
use crate::domain::path::{PathLabel, PathPolicy, PathTrace};
use crate::error::Result;
use crate::infrastructure::{NetworkArc, NetworkArcId, NetworkGraph, NetworkNode, NetworkNodeId};

pub(super) fn node_id(index: usize) -> NetworkNodeId { format!("pricing-node:{index}").into() }

pub(super) fn graph(
    node_count: usize,
    arcs: impl Iterator<Item = (NetworkArcId, usize, usize)>,
) -> Result<NetworkGraph<usize, usize>> {
    NetworkGraph::new((0..node_count).map(|index| NetworkNode::new(node_id(index), index)).collect(),
        arcs.enumerate().map(|(index, (id, from, to))| NetworkArc::new(id, node_id(from), node_id(to), index)).collect())
}

pub(super) trait PricingLabel: Clone {
    fn set_predecessor(&mut self, predecessor: Option<usize>, arc: Option<usize>);
}

#[derive(Clone)]
pub(super) struct PricingPathState<L, R> {
    pub label: L,
    pub route: Option<R>,
}

pub(super) struct PricingPathPolicy<L, E, F, D, C> {
    pub root: L,
    pub extend: E,
    pub finish: F,
    pub dominates: Option<D>,
    pub compare: C,
}

impl<L, R, E, F, D, C> PathPolicy<usize, usize, PricingPathState<L, R>> for PricingPathPolicy<L, E, F, D, C>
where
    L: PricingLabel,
    E: Fn(usize, &L, usize, usize) -> Result<Option<L>>,
    F: Fn(&[L]) -> Result<Option<(L, R)>>,
    D: Fn(&L, &L) -> bool,
    C: Fn(&L, &R, &L, &R) -> Ordering,
{
    fn start(&self, _: &NetworkNode<usize>) -> Result<Option<PricingPathState<L, R>>> {
        Ok(Some(PricingPathState { label: self.root.clone(), route: None }))
    }

    fn extend(
        &self,
        state: &PricingPathState<L, R>,
        arc: &NetworkArc<usize>,
        destination: &NetworkNode<usize>,
    ) -> Result<Option<PricingPathState<L, R>>> {
        self.transition(0, state, arc, destination)
    }

    fn extend_from_label(
        &self,
        index: usize,
        label: &PathLabel<PricingPathState<L, R>>,
        arc: &NetworkArc<usize>,
        destination: &NetworkNode<usize>,
    ) -> Result<Option<PricingPathState<L, R>>> {
        self.transition(index, &label.state, arc, destination)
    }

    fn finish(&self, trace: &PathTrace<'_, usize, usize, PricingPathState<L, R>>) -> Result<Option<PricingPathState<L, R>>> {
        // 独立回溯使用局部连续链，公开支配回调仍看到搜索池的真实前驱索引。
        // Independent backtracking uses a local contiguous chain; public dominance sees actual pool predecessors.
        let labels: Vec<_> = trace.states.iter().enumerate().map(|(index, state)| {
            let mut label = state.label.clone();
            label.set_predecessor(index.checked_sub(1), index.checked_sub(1).map(|arc| trace.arcs[arc].payload));
            label
        }).collect();
        Ok((self.finish)(&labels)?.map(|(label, route)| PricingPathState { label, route: Some(route) }))
    }

    fn has_dominance(&self) -> bool { self.dominates.is_some() }
    fn dominance_is_exact(&self) -> bool { self.dominates.is_some() }
    fn dominates(&self, left: &PricingPathState<L, R>, right: &PricingPathState<L, R>) -> bool {
        self.dominates.as_ref().is_some_and(|dominates| dominates(&left.label, &right.label))
    }

    fn compare(&self, left: &PricingPathState<L, R>, right: &PricingPathState<L, R>) -> Ordering {
        match (&left.route, &right.route) {
            (Some(a), Some(b)) => (self.compare)(&left.label, a, &right.label, b),
            _ => Ordering::Equal,
        }
    }
}

impl<L, E, F, D, C> PricingPathPolicy<L, E, F, D, C>
where
    E: Fn(usize, &L, usize, usize) -> Result<Option<L>>,
{
    fn transition<R>(
        &self,
        index: usize,
        state: &PricingPathState<L, R>,
        arc: &NetworkArc<usize>,
        destination: &NetworkNode<usize>,
    ) -> Result<Option<PricingPathState<L, R>>> {
        Ok((self.extend)(index, &state.label, arc.payload, destination.payload)?
            .map(|label| PricingPathState { label, route: None }))
    }
}
