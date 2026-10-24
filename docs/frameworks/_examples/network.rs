use ospf_rust_core::solver::LinearSolver;
use ospf_rust_framework_network_scheduling::application::FlowApplicationService;
use ospf_rust_framework_network_scheduling::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem};
use ospf_rust_framework_network_scheduling::infrastructure::{CapacityBounds, NetworkCost};
use ospf_rust_quantities::{Quantity, unit::{CTUnit, NoneUnit}};

pub fn run_example<S: LinearSolver>(solver: S) -> Result<(), String> {
    let q = |value| Quantity::new(value, NoneUnit::INSTANT.clone());
    let problem = FlowProblem::new(
        vec![FlowNode::new("source"), FlowNode::new("sink")],
        vec![FlowArc::new("delivery", "source", "sink",
            CapacityBounds::try_new(q(0.0), q(3.0)).map_err(|e| format!("{e:?}"))?,
            NetworkCost::new(q(3.0)))],
        vec![FlowCommodity::new("goods")
            .with_balance("source", q(2.0)).with_balance("sink", q(-2.0))],
    ).map_err(|e| format!("{e:?}"))?;
    let report = FlowApplicationService::new(solver).solve(&problem).map_err(|e| format!("{e:?}"))?;
    println!("report={report:?}");
    match report.solution.and_then(|solution| solution.value) {
        Some(flow) => {
            // #region result
            for ((commodity, arc), amount) in &flow.flows { println!("commodity={commodity:?}, arc={arc:?}, amount={amount:?}"); }
            println!("resource usage={:?}", flow.resources);
            println!("unused supply={:?}, unmet demand={:?}", flow.spills, flow.shortages);
            println!("operating={:?}, penalties=({:?}, {:?}), total={:?}", flow.operating_cost, flow.spill_cost, flow.shortage_cost, flow.total_cost);
            assert_eq!(flow.flows.values().next().map(|q| q.value), Some(2.0));
            // #endregion result
        },
        None => println!("No flow; retain the report and its termination reason"),
    }
    Ok(())
}
