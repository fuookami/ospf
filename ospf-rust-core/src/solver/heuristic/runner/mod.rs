//! 生命周期骨架 / Lifecycle skeletons
//!
//! 骨架只实现三个钩子：建立初始状态、推进一步、把状态摊平为个体；循环、档案、终止与观测由 `drive` 统一实现。
//! A skeleton implements only three hooks: build the initial state, advance one step, and flatten the state into
//! individuals; the loop, archive, stopping, and observation are implemented once by `drive`.

pub mod cmaes_runner;
pub mod clpso_runner;
pub mod drive;
pub mod island_runner;
pub mod iteration_observer;
pub mod nsga2_runner;
pub mod particle_swarm_runner;
pub mod population_runner;
pub mod runner_core;
pub mod shade_runner;
pub mod trajectory_runner;

pub use cmaes_runner::{CmaEsRunner, CmaEsState};
pub use clpso_runner::{ClpsoConfig, ClpsoParticle, ClpsoRunner};
pub use drive::drive;
pub use island_runner::{IslandRunner, Islands};
pub use iteration_observer::IterationObserver;
pub use nsga2_runner::{Nsga2Runner, Nsga3Runner, NsgaConfig};
pub use particle_swarm_runner::{ParticleState, ParticleSwarmRunner};
pub use population_runner::{Population, PopulationRunner};
pub use runner_core::RunnerCore;
pub use shade_runner::{ShadeConfig, ShadeMemory, ShadeParameters, ShadeRunner, ShadeState};
pub use trajectory_runner::TrajectoryRunner;
