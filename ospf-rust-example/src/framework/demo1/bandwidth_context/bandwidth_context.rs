use std::error::Error;
use ospf_rust_core::model::MetaModel;
use crate::framework::demo1::route_context::RouteContext;
use super::aggregation::Aggregation;
use super::model::{EdgeBandwidth, NodeBandwidth, ServiceBandwidth};

pub struct BandwidthContext {
    pub aggregation: Option<Aggregation>,
}

impl BandwidthContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        let mut edge_bandwidth = EdgeBandwidth::new();
        edge_bandwidth.register(
            model,
            &route_agg.graph.edges,
            &route_agg.services,
            &route_agg.graph.nodes,
        )?;

        self.aggregation = Some(Aggregation::new(
            edge_bandwidth,
            ServiceBandwidth::new(),
            NodeBandwidth::new(),
        ));
        Ok(())
    }

    pub fn construct(
        &self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_ref()
            .ok_or("bandwidth context not registered")?;
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        super::service::generate_pipelines(
            agg,
            model,
            &route_agg.graph.edges,
            &route_agg.services,
            &route_agg.graph.nodes,
            &route_agg.assignment.normal_node_indices,
            &route_agg.assignment.x_idx,
        )
    }

    pub fn analyze(
        &self,
        route_context: &RouteContext,
        solution: &[f64],
    ) -> Result<Vec<Vec<u64>>, Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_ref()
            .ok_or("bandwidth context not registered")?;
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        let analyzer = super::service::solution_analyzer::SolutionAnalyzer::new(
            &route_agg.graph,
            &route_agg.services,
            &route_agg.assignment,
            agg,
        );
        Ok(analyzer.analyze(solution))
    }
}
