//! Dimension - 量纲
//! Dimension - Physical dimensions
//!
//! 提供运行时和编译时的量纲表示和运算
//! Provides runtime and compile-time dimension representation and operations
//!
//! # 模块结构 / Module Structure
//! - `fundamental_quantity`: 基础量纲（L, M, T, I, Θ, N, J, ℐ, φ, Ω）
//! - `derived_quantity`: 导出量纲的运行时和编译时表示
//! - `derived`: 预定义的导出量纲类型（面积、速度、力等）

pub mod fundamental_quantity;
pub mod derived_quantity;
pub mod derived;

// 重导出基础量纲类型 / Re-export fundamental dimension types
pub use fundamental_quantity::{
    // 运行时类型 / Runtime types
    CTFundamentalDimension, CTFundamentalDiv, CTFundamentalMul,
    // 基础量纲类型 / Base dimension types
    CTFundamentalPow, CTFundamentalQuantity, CTFundamentalQuantityTrait, CTFundamentalReciprocal, FundamentalDimension, FundamentalQuantity, FundamentalQuantityEnum, Info, Info0, Info1,
    // 编译时类型 / Compile-time types
    Omega, Omega0,
    Omega1, Phi, Phi0, Phi1, SameFundamentalDimension,
    SameFundamentalPower, SameFundamentalQuantity, Theta,
    // 长度 L / Length L
    Theta0, Theta1, I, I0,
    // 质量 M / Mass M
    I1, J,
    // 时间 T / Time T
    J0, J1, L, L0,
    // 电流 I / Electric Current I
    L1, L2,
    // 热力学温度 Θ / Thermodynamic Temperature Θ
    L3, M,
    // 物质的量 N / Amount of Substance N
    M0, M1,
    // 发光强度 J / Luminous Intensity J
    N, N0,
    // 信息量 ℐ / Information ℐ
    N1, T,
    // 平面角 φ / Plane Angle φ
    T0, T1,
    // 立体角 Ω / Solid Angle Ω
    TN1, TN2,
};

// 重导出导出量纲类型 / Re-export derived dimension types
pub use derived_quantity::{
    // 运行时类型 / Runtime types
    CTDerivedDiv,
    // 编译时类型 / Compile-time types
    CTDerivedMul, CTDerivedPow, CTDerivedQuantity, CTDerivedReciprocal, DerivedQuantity,
    SameDerivedDimension
};

// 重导出导出量纲实现 / Re-export derived dimension implementations
pub use derived::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensionless_operations() {
        // 长度 / 长度 = 无量纲
        // Length / Length = None
        let length = DerivedQuantity::from_base("".to_string(), FundamentalQuantityEnum::Length);
        let none1 = (length.clone() / length).build();
        let none2 = DerivedQuantity::none("".to_string());
        assert!(none1.is_none());
        assert_eq!(none1, none2);
    }

    #[test]
    fn test_length_dimension() {
        // 验证编译时 Length 量纲符号 / Verify compile-time Length dimension symbol
        assert_eq!(*Length::SYMBOL, "L");
    }

    #[test]
    fn test_same_dimension_trait() {
        fn assert_same_dim<D1: CTDerivedQuantity, D2: CTDerivedQuantity>()
        where
            D1: SameDerivedDimension<D2>,
        {
        }

        // 无量纲和无量纲相同
        // Dimensionless and dimensionless are the same
        assert_same_dim::<DimLess, DimLess>();
    }
}
