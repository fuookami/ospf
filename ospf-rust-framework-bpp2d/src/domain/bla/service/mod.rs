//! BLA 服务 / BLA services
//!
//! 底部向上左对齐排样。
//! Bottom-up left-justified layout.

pub mod bottom_up_left_justified_algorithm;

pub use bottom_up_left_justified_algorithm::{
    Bla2dLayoutResult, BlaConfig, BottomUpLeftJustifiedAlgorithm, compare_placement_position,
};
