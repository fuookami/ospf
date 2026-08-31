use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::UContinuousVariableItem;
use crate::framework::demo1::route_context::model::{Edge, Node, NodeKind, Service};

pub struct EdgeBandwidth {
    pub y_idx: Vec<Vec<usize>>,
}

impl EdgeBandwidth {
    pub fn new() -> Self {
        Self { y_idx: Vec::new() }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        edges: &[Edge],
        services: &[Service],
        _nodes: &[Node],
    ) -> Result<(), Box<dyn Error>> {
        let mut y_idx = vec![vec![0usize; services.len()]; edges.len()];
        for (e, _edge) in edges.iter().enumerate() {
            for s in 0..services.len() {
                let variable = UContinuousVariableItem::auto(&format!("y_{}_{}", e, s));
                y_idx[e][s] = model.register_variable(variable)?;
            }
        }
        self.y_idx = y_idx;
        Ok(())
    }
}
