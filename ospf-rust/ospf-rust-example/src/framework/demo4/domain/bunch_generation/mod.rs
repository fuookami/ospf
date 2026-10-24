//! 编组生成领域模块 / Bunch generation domain module.
/// 基于共享资源路径的无损恢复束生成 / Lossless recovery bunch generation using shared resource paths.
pub mod recovery;
/// 不可变恢复活动域及完整候选回放。 / Immutable recovery activity domains and complete candidate replay.
pub mod problem;
/// Gantt 请求到共享路径定价的适配。 / Adapter from Gantt requests to shared-path pricing.
pub mod pricing;
/// 原始束与锁定域驱动的自动初始列。 / Automatic initial columns driven by original bunches and locked domains.
pub mod initial;
