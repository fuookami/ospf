use std::error::Error;
use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::variable::BinaryVariableItem;
use crate::framework::demo1::infrastructure::dto::Input;
use super::model::{Assignment, Edge, Node, Service};

pub struct RouteContext {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub services: Vec<Service>,
    pub assignment: Option<Assignment>,
}

impl RouteContext {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            services: Vec::new(),
            assignment: None,
        }
    }

    pub fn init(&mut self, input: &Input) -> Result<(), Box<dyn Error>> {
        self.nodes.clear();
        self.edges.clear();
        self.services.clear();
        self.assignment = None;

        let total_demand: f64 = input.client_nodes.iter().map(|c| c.demand as f64).sum();
        let service_count = (input.normal_node_amount / 2).max(1);
        for _ in 0..service_count {
            self.services.push(Service {
                capacity: total_demand,
                cost: input.service_cost as f64,
            });
        }

        for id in 0..input.normal_node_amount {
            self.nodes.push(Node {
                id: id as u64,
                is_client: false,
                demand: 0.0,
            });
        }

        for client in &input.client_nodes {
            self.nodes.push(Node {
                id: client.id,
                is_client: true,
                demand: client.demand as f64,
            });
        }

        for edge in &input.edges {
            self.edges.push(Edge {
                from: edge.from_node_id as usize,
                to: edge.to_node_id as usize,
                max_bandwidth: edge.max_bandwidth as f64,
                cost_per_bandwidth: edge.cost_per_bandwidth as f64,
            });
            self.edges.push(Edge {
                from: edge.to_node_id as usize,
                to: edge.from_node_id as usize,
                max_bandwidth: edge.max_bandwidth as f64,
                cost_per_bandwidth: edge.cost_per_bandwidth as f64,
            });
        }

        for (offset, client) in input.client_nodes.iter().enumerate() {
            let client_index = input.normal_node_amount + offset;
            self.edges.push(Edge {
                from: client.normal_node_id as usize,
                to: client_index,
                max_bandwidth: client.demand as f64,
                cost_per_bandwidth: 0.0,
            });
        }
        Ok(())
    }

    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn Error>> {
        let normal_node_indices: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| if node.is_client { None } else { Some(idx) })
            .collect();

        let mut x_idx = vec![vec![0usize; self.services.len()]; normal_node_indices.len()];
        for (row, node_idx) in normal_node_indices.iter().enumerate() {
            for (s, _) in self.services.iter().enumerate() {
                let variable = BinaryVariableItem::auto(&format!("x_{}_{}", node_idx, s));
                x_idx[row][s] = model.register_variable(variable)?;
            }
        }

        self.assignment = Some(Assignment {
            normal_node_indices,
            x_idx,
        });
        Ok(())
    }

    pub fn construct(&self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn Error>> {
        let assignment = self
            .assignment
            .as_ref()
            .ok_or_else(|| String::from("route context not registered"))?;

        for (row, _) in assignment.normal_node_indices.iter().enumerate() {
            let coefficients: Vec<(usize, f64)> = self
                .services
                .iter()
                .enumerate()
                .map(|(s, _)| (assignment.x_idx[row][s], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("node_assignment_{}", row),
            )?;
        }

        for (s, _) in self.services.iter().enumerate() {
            let coefficients: Vec<(usize, f64)> = assignment
                .normal_node_indices
                .iter()
                .enumerate()
                .map(|(row, _)| (assignment.x_idx[row][s], 1.0))
                .collect();
            model.add_linear_constraint(
                &coefficients,
                ConstraintRelation::LessEqual,
                1.0,
                &format!("service_assignment_{}", s),
            )?;
        }

        let objective: Vec<(usize, f64)> = assignment
            .normal_node_indices
            .iter()
            .enumerate()
            .flat_map(|(row, _)| {
                self.services
                    .iter()
                    .enumerate()
                    .map(move |(s, service)| (assignment.x_idx[row][s], service.cost))
            })
            .collect();
        model.add_linear_objective(&objective, "service_cost");
        Ok(())
    }

    pub fn assignment_variable(&self, normal_node_idx: usize, service_idx: usize) -> Option<usize> {
        let assignment = self.assignment.as_ref()?;
        let row = assignment
            .normal_node_indices
            .iter()
            .position(|idx| *idx == normal_node_idx)?;
        Some(assignment.x_idx[row][service_idx])
    }
}
