#![allow(unused_imports, dead_code)]
//! 框架示例模块 / Framework examples module.

pub mod demo1;
pub mod demo2;
pub mod demo3;
pub mod demo4;

pub use demo1::run as run_demo1;
pub use demo2::run as run_demo2;
pub use demo3::run as run_demo3;
pub use demo4::run as run_demo4;
