//! CSP2D 基础设施 / CSP2D infrastructure
//!
//! 裁切区域、刀缝、断头台刀线与切分树、裁切工艺与错误类型。共享二维几何来自
//! `ospf-rust-framework-geometry2d`，本层只做再导出。
//! Cutting regions, kerf, guillotine cut lines and trees, cutting processes and error types. The shared 2D
//! geometry comes from `ospf-rust-framework-geometry2d`; this layer only re-exports it.

pub mod cut_tree;
pub mod cutting_process;
pub mod error;
pub mod region;

pub use cut_tree::{CutLine2, CutNode2, CutTree2, Leaf2, has_positive_area, is_inside};
pub use cutting_process::CuttingProcess2;
pub use error::{
    Csp2dCapabilityError, Csp2dInternalError, Csp2dResult, Csp2dSolvingError, Csp2dValidationError,
    capability_error, solving_error, validation_error,
};
pub use region::{CutOrientation2, Region2, region2_of};

pub use ospf_rust_framework_geometry2d::{
    Box2Geometry, Placement2, PackingShape2, QuantityPoint2, Sheet2Shape, area_of,
    quantity_greater_or_equal, quantity_is_positive, quantity_less_or_equal, quantity_multiply,
    quantity_zero_of,
};
