//! 代数结构概念模块
//! Algebraic structure concept module
//!
//! 本模块定义了完整的代数结构层次，包括：
//! This module defines a complete hierarchy of algebraic structures, including:
//!
//! # 基础代数结构 / Basic Algebraic Structures
//! - [`Semigroup`] - 半群（结合律）/ Semigroup (associativity)
//! - [`Monoid`] - 幺半群（半群 + 单位元）/ Monoid (semigroup + identity)
//! - [`Group`] - 群（幺半群 + 逆元）/ Group (monoid + inverse)
//! - [`AbelianGroup`] - 阿贝尔群（群 + 交换律）/ Abelian group (group + commutativity)
//!
//! # 乘法结构 / Multiplicative Structures
//! - [`MultiplicativeSemigroup`] - 乘法半群
//! - [`MultiplicativeMonoid`] - 乘法幺半群
//! - [`MultiplicativeGroup`] - 乘法群
//!
//! # 环与域 / Rings and Fields
//! - [`Ring`] - 环（加法群 + 乘法半群）/ Ring (additive group + multiplicative semigroup)
//! - [`CommutativeRing`] - 交换环（环 + 乘法交换律）/ Commutative ring (ring + multiplicative commutativity)
//! - [`Field`] - 域（交换环 + 乘法逆元）/ Field (commutative ring + multiplicative inverse)
//!
//! # 线性代数结构 / Linear Algebra Structures
//! - [`VectorSpace`] - 向量空间（使用 GAT 定义标量域）/ Vector space (using GAT for scalar field)
//! - [`NormedSpace`] - 赋范空间（向量空间 + 范数）/ Normed space (vector space + norm)
//! - [`InnerProductSpace`] - 内积空间（赋范空间 + 内积）/ Inner product space (normed space + inner product)
//!
//! # 有序结构 / Ordered Structures
//! - [`TotallyOrdered`] - 全序
//!
//! # 其他性质 / Other Properties
//! - [`Bounded`] - 有界性
//! - [`Epsilon`] - 默认精度容差
//! - [`Fixed`] - 固定性
//! - [`Infinite`] - 无穷大支持

// ============================================================================
// 模块声明 / Module declarations
// 注意：顺序很重要，需要按照依赖关系排列
// Note: Order matters, must be arranged by dependency
// ============================================================================

// 基础代数结构 / Basic algebraic structures
pub mod abelian_group;
pub mod group;
pub mod monoid;
pub mod semigroup;

// 乘法结构 / Multiplicative structures
pub mod multiplicative_group;
pub mod multiplicative_monoid;
pub mod multiplicative_semigroup;

// 环与域 / Rings and fields
pub mod commutative_ring;
pub mod field;
pub mod ring;

// 线性代数结构 / Linear algebra structures
pub mod inner_product_space;
pub mod normed_space;
pub mod vector_space;

// 有序结构 / Ordered structures
pub mod totally_ordered;

// 其他性质 / Other properties
pub mod bounded;
pub mod epsilon;
pub mod fixed;
pub mod infinite;

// 标量类型标记 / Scalar type marker
pub mod scalar;

// ============================================================================
// Re-exports
// ============================================================================

// 基础代数结构 / Basic algebraic structures
pub use abelian_group::AbelianGroup;
pub use group::Group;
pub use monoid::Monoid;
pub use semigroup::Semigroup;

// 乘法结构 / Multiplicative structures
pub use multiplicative_group::MultiplicativeGroup;
pub use multiplicative_monoid::MultiplicativeMonoid;
pub use multiplicative_semigroup::MultiplicativeSemigroup;

// 环与域 / Rings and fields
pub use commutative_ring::CommutativeRing;
pub use field::Field;
pub use ring::Ring;

// 线性代数结构 / Linear algebra structures
pub use inner_product_space::InnerProductSpace;
pub use normed_space::NormedSpace;
pub use vector_space::VectorSpace;

// 有序结构 / Ordered structures
pub use totally_ordered::TotallyOrdered;

// 其他性质 / Other properties
pub use bounded::Bounded;
pub use epsilon::Epsilon;
pub use fixed::Fixed;
pub use infinite::Infinite;

// 标量类型标记 / Scalar type marker
pub use scalar::Scalar;

// 引用操作约束 (从各模块导出) / Reference operation constraints (exported from modules)
pub use abelian_group::AbelianGroupRef;
pub use commutative_ring::CommutativeRingRef;
pub use field::FieldRef;
pub use group::GroupRef;
pub use monoid::MonoidRef;
pub use multiplicative_group::MultiplicativeGroupRef;
pub use multiplicative_monoid::MultiplicativeMonoidRef;
pub use multiplicative_semigroup::MultiplicativeSemigroupRef;
pub use ring::RingRef;
pub use semigroup::SemigroupRef;
