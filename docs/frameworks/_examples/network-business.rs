use ospf_rust_core::solver::LinearSolver;
use ospf_rust_framework_network_scheduling::application::FlowApplicationService;
use ospf_rust_framework_network_scheduling::domain::flow::{FlowArc, FlowCommodity, FlowNode, FlowProblem, FlowResource, FlowBalanceSlack};
use ospf_rust_framework_network_scheduling::infrastructure::{CapacityBounds, NetworkCost};
use ospf_rust_quantities::{Quantity, unit::{CTUnit, NoneUnit}};

pub fn make_problem(dock_capacity: f64) -> Result<FlowProblem<f64>, String> {
    let q = |value| Quantity::new(value, NoneUnit::INSTANT.clone());
    // #region input
    let arcs = vec![FlowArc::new("delivery", "source", "sink",
        CapacityBounds::try_new(q(0.0), q(3.0)).map_err(|e| format!("{e:?}"))?, NetworkCost::new(q(3.0)))];
    let goods = FlowCommodity::new("goods").with_balance("source", q(2.0)).with_balance("sink", q(-2.0));
    let problem = FlowProblem::new(vec![FlowNode::new("source"), FlowNode::new("sink")], arcs, vec![goods])
        .map_err(|e| format!("{e:?}"))?;
    // #endregion input
    // #region rule
    let dock = FlowResource { id: "dock".into(),
        capacity: CapacityBounds::try_new(q(0.0), q(dock_capacity)).map_err(|e| format!("{e:?}"))?,
        consumptions: [(("goods".into(), "delivery".into()), q(1.0))].into_iter().collect() };
    problem.with_resources(vec![dock]).map_err(|e| format!("{e:?}"))
    // #endregion rule
}

pub fn make_slack_problem() -> Result<FlowProblem<f64>, String> {
    let q = |value| Quantity::new(value, NoneUnit::INSTANT.clone());
    let mut goods = FlowCommodity::new("goods").with_balance("source", q(3.0)).with_balance("sink", q(-4.0));
    // #region slack
    goods.slacks.insert("source".into(), FlowBalanceSlack { spill_limit: q(1.0), shortage_limit: q(0.0),
        spill_cost: NetworkCost::new(q(5.0)), shortage_cost: NetworkCost::new(q(0.0)) });
    goods.slacks.insert("sink".into(), FlowBalanceSlack { spill_limit: q(0.0), shortage_limit: q(2.0),
        spill_cost: NetworkCost::new(q(0.0)), shortage_cost: NetworkCost::new(q(7.0)) });
    FlowProblem::new(vec![FlowNode::new("source"), FlowNode::new("sink")], vec![FlowArc::new("delivery", "source", "sink",
        CapacityBounds::try_new(q(0.0), q(2.0)).map_err(|e| format!("{e:?}"))?, NetworkCost::new(q(1.0)))], vec![goods])
        .map_err(|e| format!("{e:?}"))
    // #endregion slack
}

// #region entry
pub fn run_business<S: LinearSolver>(solver: S, dock_capacity: f64) -> Result<(), String> {
    let report = FlowApplicationService::new(solver).solve(&make_problem(dock_capacity)?).map_err(|e| format!("{e:?}"))?;
    println!("{report:?}");
    Ok(())
}
// #endregion entry
