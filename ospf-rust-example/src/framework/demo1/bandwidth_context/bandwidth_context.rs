use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::UContinuousVariableItem;
use crate::framework::demo1::route_context::RouteContext;

pub struct BandwidthContext {
    pub y_idx: Option<Vec<Vec<usize>>>,
}

impl BandwidthContext {
    pub fn new() -> Self {
        Self { y_idx: None }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let mut y_idx = vec![vec![0usize; route_context.services.len()]; route_context.edges.len()];
        for (e, _) in route_context.edges.iter().enumerate() {
            for (s, _) in route_context.services.iter().enumerate() {
                let variable = UContinuousVariableItem::auto(&format!("y_{}_{}", e, s));
                y_idx[e][s] = model.register_variable(variable)?;
            }
        }
        self.y_idx = Some(y_idx);
        Ok(())
    }

    pub fn construct(
        &self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let y_idx = self
            .y_idx
            .as_ref()
            .ok_or_else(|| String::from("bandwidth context not registered"))?;

        for (e, edge) in route_context.edges.iter().enumerate() {
            let coefficients: Vec<(usize, f64)> = route_context
                .services
                .iter()
                .enumerate()
                .map(|(s, _)| (y_idx[e][s], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                edge.max_bandwidth,
                &format!("edge_bandwidth_{}", e),
            )?;
        }

        for (e, edge) in route_context.edges.iter().enumerate() {
            for (s, _) in route_context.services.iter().enumerate() {
                if let Some(x_idx) = route_context.assignment_variable(edge.from, s) {
                    model.add_linear_constraint(
                        &[(y_idx[e][s], 1.0), (x_idx, -edge.max_bandwidth)],
                        ConstraintRelation::LessEqual,
                        0.0,
                        &format!("service_gate_{}_{}", e, s),
                    )?;
                }
            }
        }

        for (node_idx, node) in route_context.nodes.iter().enumerate() {
            if !node.is_client {
                continue;
            }
            let coefficients: Vec<(usize, f64)> = route_context
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| edge.to == node_idx)
                .flat_map(|(e, _)| {
                    route_context
                        .services
                        .iter()
                        .enumerate()
                        .map(move |(s, _)| (y_idx[e][s], 1.0))
                })
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::GreaterEqual,
                node.demand,
                &format!("demand_{}", node.id),
            )?;
        }

        for (s, service) in route_context.services.iter().enumerate() {
            let coefficients: Vec<(usize, f64)> = route_context
                .edges
                .iter()
                .enumerate()
                .filter(|(_, edge)| !route_context.nodes[edge.from].is_client)
                .map(|(e, _)| (y_idx[e][s], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                service.capacity,
                &format!("service_capacity_{}", s),
            )?;
        }

        let objective: Vec<(usize, f64)> = route_context
            .edges
            .iter()
            .enumerate()
            .flat_map(|(e, edge)| {
                route_context
                    .services
                    .iter()
                    .enumerate()
                    .map(move |(s, _)| (y_idx[e][s], edge.cost_per_bandwidth))
            })
            .collect();
        model.add_linear_objective(&objective, "bandwidth_cost");
        Ok(())
    }

    pub fn analyze(
        &self,
        route_context: &RouteContext,
        solution: &[f64],
    ) -> Result<Vec<Vec<u64>>, Box<dyn Error>> {
        let y_idx = self
            .y_idx
            .as_ref()
            .ok_or_else(|| String::from("bandwidth context not registered"))?;
        let mut links = Vec::new();
        for (e, edge) in route_context.edges.iter().enumerate() {
            for (s, _) in route_context.services.iter().enumerate() {
                let idx = y_idx[e][s];
                let value = solution.get(idx).copied().unwrap_or(0.0);
                if value > 1e-6 {
                    let from_id = route_context.nodes[edge.from].id;
                    let to_id = route_context.nodes[edge.to].id;
                    links.push(vec![from_id, to_id]);
                }
            }
        }
        Ok(links)
    }
}
