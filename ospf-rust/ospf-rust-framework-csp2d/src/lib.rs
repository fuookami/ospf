//! 二维套裁领域框架 / CSP2D domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-csp2d` 的 Rust 迁移：二维套裁的双供料模式
//! （卷材条带排样与定尺板材装箱）、断头台切分树、图案生成与列生成主问题。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-csp2d`: the dual supply modes of 2D
//! cutting stock (coil strip packing and fixed-sheet packing), guillotine cut trees, pattern generation and the
//! column-generation master problem.
//!
//! 二维几何与 BPP2D 共用 `ospf-rust-framework-geometry2d`。
//! The 2D geometry is shared with BPP2D through `ospf-rust-framework-geometry2d`.

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::{
    Csp2dConfiguration, Csp2dProblem, Csp2dSolution, Csp2dSolvers, Csp2dTerminationReason,
};
pub use domain::material::{
    Costar2, CuttingPlan2, DemandMode2, DemandValue2, Machine2, Material2, Material2Kind,
    Pattern2, Product2, ProductDemand2, ProductDemands2,
};
pub use infrastructure::{
    Box2Geometry, Csp2dCapabilityError, Csp2dInternalError, Csp2dResult, Csp2dSolvingError,
    Csp2dValidationError, CutLine2, CutNode2, CutOrientation2, CutTree2, CuttingProcess2, Leaf2,
    Region2, capability_error, region2_of, solving_error, validation_error,
};
