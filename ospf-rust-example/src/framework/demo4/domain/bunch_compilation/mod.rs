//! 编组编制领域模块 / Bunch compilation domain module.
/// 编组编制上下文 / Bunch compilation context
pub mod context;
/// 实际终点机队和缺口费用组件。 / Actual terminal fleet and shortage-cost component.
pub mod fleet_balance;
/// 链接拆分及飞机闲置费用组件。 / Link-split and aircraft-idle cost component.
pub mod operating_costs;
/// 限制使用量与费用的受管模型组件。 / Managed restriction usage and cost component.
pub mod restrictions;
/// 有限机场事件/停场容量及超额费用组件。 / Finite airport-event/stay capacity and overage-cost component.
pub mod flow_control;
/// 同源业务行及主问题组件。 / Shared business rows and master component.
pub mod rules;
/// 编组编制服务 / Bunch compilation service
pub mod service;
