//! 二维图案生成领域 / 2D pattern generation domain
//!
//! 映射 Kotlin `csp2d-domain-pattern-generation-context`：图案生成请求、货架式生成器与
//! 断头台切分树构造。检验数定价留待后续阶段，本层不声称已实现 reduced-cost pricing。
//! Maps the Kotlin `csp2d-domain-pattern-generation-context` submodule: the generation request, the shelf
//! generator and guillotine cut-tree construction. Reduced-cost pricing is left to a later stage and is not
//! claimed here.

pub mod costar_filler;
pub mod request;
pub mod shelf_generator;

pub use costar_filler::{CostarFiller, CostarFillingResult};
pub use request::{PatternGenerationRequest2, PatternGenerator2};
pub use shelf_generator::{ShelfPatternGenerator2, shelf_pattern};
