//! # ospf-rust-quantities
//!
//! 物理量、量纲和单位系统
//! Physical quantities, dimensions and units system
//!
//! 支持运行时和编译时的量纲运算
//! Supports both runtime and compile-time dimension operations
//!
//! # 核心类型 / Core Types
//! - `Quantity<V, U>`: 统一的物理量类型
//!   - `Quantity<V, Unit>`: 运行时物理量
//!   - `Quantity<V, U: CTUnit>`: 编译时物理量（零成本抽象）
//! - `QuantityTrait`: 物理量统一接口

pub mod dimension;
pub mod error;
pub mod functional;
pub mod quantity;
pub mod scale;
pub mod unit;

// 重导出常用类型 / Re-export common types
pub use error::{DimensionMismatchError, SymbolRegistryError, UnitConversionError};
pub use unit::conversion_value::{UnitConversionCalculation, UnitConversionValue};

// 从 quantity 模块重导出 / Re-export from quantity module
pub use functional::*;
pub use quantity::{Quantity, QuantityTrait};

// ============================================================================
// 自定义量纲测试 / Custom dimension tests
// ============================================================================

#[cfg(test)]
mod custom_dimension_tests {
    use super::*;
    use crate::dimension::{
        CustomFundamentalDimension, DerivedQuantity, FundamentalDimension, FundamentalQuantityEnum,
    };
    use crate::quantity::QuantityTrait;
    use crate::scale::Scale;
    use crate::unit::{Unit, UnitBuilder, system::SI_SYSTEM};
    use bigdecimal::BigDecimal;
    use std::sync::Arc;

    // ========================================================================
    // 辅助函数 / Helper functions
    // ========================================================================

    /// 创建自定义量纲
    /// Create custom dimension
    fn create_custom_dimension(dimension: FundamentalQuantityEnum, name: &str) -> DerivedQuantity {
        DerivedQuantity::from_base(name.to_string(), dimension)
    }

    /// 创建自定义单位
    /// Create custom unit
    fn create_custom_unit(
        name: &str,
        symbol: &str,
        dimension: DerivedQuantity,
        scale: Scale,
    ) -> Unit {
        Unit::new(name.to_string(), symbol.to_string(), dimension, scale)
    }

    // ========================================================================
    // 测试：自定义量纲创建 / Test: Custom dimension creation
    // ========================================================================

    #[test]
    fn test_custom_dimension() {
        // 1. 定义"卷"的量纲（使用 CustomFundamentalDimension）
        // Define "Tome" dimension (using CustomFundamentalDimension)
        let tome = CustomFundamentalDimension::new("T");
        let tome_dimension = tome.to_enum();
        let tome_quantity = create_custom_dimension(tome_dimension.clone(), "tome");

        // 验证自定义量纲的符号
        // Verify custom dimension symbol
        assert_eq!(tome.symbol(), "T");
        assert_eq!(tome_quantity.symbol(), "T");

        // 2. 定义"卷"单位
        // Define "Tome" unit
        let tome_unit = create_custom_unit("tome", "T", tome_quantity.clone(), Scale::new());

        // 验证单位
        // Verify unit
        assert_eq!(tome_unit.symbol(), "T");
        assert_eq!(tome_unit.name(), "tome");

        // 3. 定义"卷每千克"单位（卷/质量）
        // Define "Tome per kilogram" unit (tome/mass)
        let mass_quantity =
            DerivedQuantity::from_base("mass".to_string(), FundamentalQuantityEnum::Mass);
        let tome_per_kg_quantity = (&tome_quantity / &mass_quantity).build();

        let tome_per_kg_unit = create_custom_unit(
            "tome per kilogram",
            "T/kg",
            tome_per_kg_quantity,
            Scale::new(),
        );

        // 验证卷每千克的量纲符号（应该包含 T 和 M）
        // Verify tome per kilogram dimension symbol (should contain T and M)
        let expected_symbol = tome_per_kg_unit.dimension().symbol();
        assert!(
            expected_symbol.contains("T"),
            "Expected symbol to contain 'T', got: {}",
            expected_symbol
        );
        assert!(
            expected_symbol.contains("M"),
            "Expected symbol to contain 'M', got: {}",
            expected_symbol
        );

        // 4. 创建 3 卷的物理量
        // Create 3 tome physical quantity
        let three_tome = Quantity::new(BigDecimal::from(3), tome_unit.clone());
        assert_eq!(three_tome.value, BigDecimal::from(3));

        // 5. 测试相同单位的加法
        // Test addition with same unit
        let another_three_tome = Quantity::new(BigDecimal::from(3), tome_unit.clone());
        let sum = &three_tome + &another_three_tome;
        assert_eq!(sum.value, BigDecimal::from(6));

        // 6. 测试相同单位的除法
        // Test division with same unit
        let six_tome = Quantity::new(BigDecimal::from(6), tome_unit.clone());
        let two_tome = Quantity::new(BigDecimal::from(2), tome_unit);
        let quotient = &six_tome / &two_tome;
        assert_eq!(quotient.value, BigDecimal::from(3));
        // 验证结果是无量纲（量纲符号为 "1"）
        // Verify result is dimensionless (dimension symbol is "1")
        let quotient_symbol = quotient.unit.dimension().symbol();
        assert_eq!(
            quotient_symbol, "1",
            "Expected dimensionless, got: {}",
            quotient_symbol
        );
    }

    // ========================================================================
    // 测试：卷量纲的计算 / Test: Tome dimension calculations
    // ========================================================================

    #[test]
    fn test_tome_dimension_calculations() {
        // 1. 定义"卷"量纲（使用 CustomFundamentalDimension）
        // Define "Tome" dimension (using CustomFundamentalDimension)
        let tome = CustomFundamentalDimension::new("T");
        let tome_dimension = tome.to_enum();
        let tome_quantity = create_custom_dimension(tome_dimension.clone(), "tome");

        // 2. 定义"卷"单位（卷量纲的标准单位）
        // Define "Tome" unit (standard unit for tome dimension)
        let tome_unit = create_custom_unit("tome", "T", tome_quantity.clone(), Scale::new());

        // 3. 定义"千克每卷"单位（质量/卷）
        // Define "Kilogram per Tome" unit (mass/tome)
        let mass_quantity =
            DerivedQuantity::from_base("mass".to_string(), FundamentalQuantityEnum::Mass);
        let kg_per_tome_quantity = (&mass_quantity / &tome_quantity).build();
        let kilogram_per_tome = create_custom_unit(
            "kilogram per tome",
            "kg/T",
            kg_per_tome_quantity,
            Scale::new(),
        );

        // 4. 测试：6 千克 / 2 千克每卷 = 3 卷
        // Test: 6 kg / 2 (kg/T) = 3 T
        // 量纲计算: M ÷ (M/T) = M × T/M = T
        let kilogram_unit =
            create_custom_unit("kilogram", "kg", mass_quantity.clone(), Scale::new());
        let six_kg = Quantity::new(BigDecimal::from(6), kilogram_unit);
        let two_kg_per_tome = Quantity::new(BigDecimal::from(2), kilogram_per_tome.clone());
        let result1 = &six_kg / &two_kg_per_tome;

        assert_eq!(result1.value, BigDecimal::from(3));
        assert!(
            result1.unit.dimension().symbol().contains("T"),
            "Expected dimension symbol to contain 'T', got: {}",
            result1.unit.dimension().symbol()
        );

        // 5. 测试：9 卷 * 3 千克每卷 = 27 千克
        // Test: 9 T * 3 (kg/T) = 27 kg
        // 量纲计算: T × (M/T) = M
        let nine_tome = Quantity::new(BigDecimal::from(9), tome_unit.clone());
        let three_kg_per_tome = Quantity::new(BigDecimal::from(3), kilogram_per_tome);
        let result2 = &nine_tome * &three_kg_per_tome;

        assert_eq!(result2.value, BigDecimal::from(27));
        assert!(
            result2.unit.dimension().symbol().contains("M"),
            "Expected dimension symbol to contain 'M', got: {}",
            result2.unit.dimension().symbol()
        );

        // 6. 测试：3 卷 不等于 3（无量纲）
        // Test: 3 T != 3 (dimensionless)
        // 有量纲值不等于无量纲值
        let three_tome = Quantity::new(BigDecimal::from(3), tome_unit);
        let dimensionless_quantity = DerivedQuantity::none("dimensionless".to_string());
        let none_unit =
            create_custom_unit("dimensionless", "1", dimensionless_quantity, Scale::new());
        let three_dimensionless = Quantity::new(BigDecimal::from(3), none_unit);

        // 由于量纲不同，eq 应该返回 false
        // eq should return false due to different dimensions
        assert_ne!(
            three_tome.unit.dimension(),
            three_dimensionless.unit.dimension()
        );

        // 7. 测试：9 卷 / 3 千克每卷 不等于 27 千克
        // Test: 9 T / 3 (kg/T) != 27 kg
        // 量纲计算: T ÷ (M/T) = T × T/M = T²/M ≠ M
        let result3 = &nine_tome / &three_kg_per_tome;

        // 验证结果值是 3
        // Verify result value is 3
        assert_eq!(result3.value, BigDecimal::from(3));
        // 验证量纲不等于质量
        // Verify dimension is not mass
        assert_ne!(result3.unit.dimension(), &mass_quantity);
    }

    // ========================================================================
    // 测试：张和令的计算 / Test: Zhang and Ling dimension calculations
    // ========================================================================

    #[test]
    fn test_zhang_and_ling_dimension_calculations() {
        // 1. 定义"张"量纲（使用 CustomFundamentalDimension）
        // Define "Zhang" dimension (using CustomFundamentalDimension)
        let zhang = CustomFundamentalDimension::new("Z");
        let zhang_dimension = zhang.to_enum();
        let zhang_quantity = create_custom_dimension(zhang_dimension.clone(), "zhang");

        // 2. 定义"令"量纲（使用 CustomFundamentalDimension）
        // Define "Ling" dimension (using CustomFundamentalDimension)
        let ling = CustomFundamentalDimension::new("L");
        let ling_dimension = ling.to_enum();
        let ling_quantity = create_custom_dimension(ling_dimension.clone(), "ling");

        // 3. 定义"张"单位（张量纲的标准单位）
        // Define "Zhang" unit (standard unit for zhang dimension)
        let zhang_unit = create_custom_unit("zhang", "Z", zhang_quantity.clone(), Scale::new());

        // 4. 定义"令"单位（令量纲的标准单位）
        // Define "Ling" unit (standard unit for ling dimension)
        let ling_unit = create_custom_unit("ling", "L", ling_quantity.clone(), Scale::new());

        // 5. 定义"张每令"单位（张/令）
        // Define "Zhang per Ling" unit (zhang/ling)
        let zhang_per_ling_quantity = (&zhang_quantity / &ling_quantity).build();
        let zhang_per_ling = create_custom_unit(
            "zhang per ling",
            "Z/L",
            zhang_per_ling_quantity,
            Scale::new(),
        );

        // 6. 测试：3 令 * 3 张每令 = 9 张
        // Test: 3 L * 3 (Z/L) = 9 Z
        // 量纲计算: L × (Z/L) = Z
        let three_ling = Quantity::new(BigDecimal::from(3), ling_unit.clone());
        let three_zhang_per_ling = Quantity::new(BigDecimal::from(3), zhang_per_ling.clone());
        let result1 = &three_ling * &three_zhang_per_ling;

        assert_eq!(result1.value, BigDecimal::from(9));
        // 验证结果量纲包含 Z（张）
        // Verify result dimension contains Z (zhang)
        assert!(
            result1.unit.dimension().symbol().contains("Z"),
            "Expected dimension symbol to contain 'Z', got: {}",
            result1.unit.dimension().symbol()
        );

        // 7. 测试：18 张 / 6 张每令 = 3 令
        // Test: 18 Z / 6 (Z/L) = 3 L
        // 量纲计算: Z ÷ (Z/L) = Z × L/Z = L
        let eighteen_zhang = Quantity::new(BigDecimal::from(18), zhang_unit.clone());
        let six_zhang_per_ling = Quantity::new(BigDecimal::from(6), zhang_per_ling);
        let result2 = &eighteen_zhang / &six_zhang_per_ling;

        assert_eq!(result2.value, BigDecimal::from(3));
        // 验证结果量纲包含 L（令）
        // Verify result dimension contains L (ling)
        assert!(
            result2.unit.dimension().symbol().contains("L"),
            "Expected dimension symbol to contain 'L', got: {}",
            result2.unit.dimension().symbol()
        );

        // 8. 测试：3 张 不等于 3 令
        // Test: 3 Z != 3 L
        // 不同量纲的值不相等
        // Values with different dimensions are not equal
        let three_zhang = Quantity::new(BigDecimal::from(3), zhang_unit);

        // 验证张和令是不同的量纲
        // Verify zhang and ling are different dimensions
        assert_ne!(zhang_quantity, ling_quantity);

        // 验证 3 张 和 3 令 量纲不同
        // Verify 3 Z and 3 L have different dimensions
        assert_ne!(three_zhang.unit.dimension(), three_ling.unit.dimension());
    }

    // ========================================================================
    // 测试：自定义量纲算术运算 / Test: Custom dimension arithmetic
    // ========================================================================

    #[test]
    fn test_custom_dimension_arithmetic() {
        // 1. 定义"卷"的量纲（使用 CustomFundamentalDimension）
        // Define "Tome" dimension (using CustomFundamentalDimension)
        let tome = CustomFundamentalDimension::new("T");
        let tome_dimension = tome.to_enum();
        let tome_quantity = create_custom_dimension(tome_dimension.clone(), "tome");

        // 2. 定义"卷"单位
        // Define "Tome" unit
        let tome_unit = create_custom_unit("tome", "T", tome_quantity.clone(), Scale::new());

        // 3. 定义"卷每千克"单位（卷/质量）
        // Define "Tome per kilogram" unit (tome/mass)
        let mass_quantity =
            DerivedQuantity::from_base("mass".to_string(), FundamentalQuantityEnum::Mass);
        let tome_per_kg_quantity = (&tome_quantity / &mass_quantity).build();
        let tome_per_kilogram = create_custom_unit(
            "tome per kilogram",
            "T/kg",
            tome_per_kg_quantity,
            Scale::new(),
        );

        // 4. 测试乘法：3 千克 × 3 卷每千克 = 9 卷
        // Test multiplication: 3 kg × 3 T/kg = 9 T
        let kilogram_unit =
            create_custom_unit("kilogram", "kg", mass_quantity.clone(), Scale::new());
        let three_kg = Quantity::new(BigDecimal::from(3), kilogram_unit);
        let three_tome_per_kg = Quantity::new(BigDecimal::from(3), tome_per_kilogram.clone());
        let product = &three_kg * &three_tome_per_kg;

        // 验证结果值为 9
        // Verify result value is 9
        assert_eq!(product.value, BigDecimal::from(9));
        // 验证结果量纲包含 T（卷）
        // Verify result dimension contains T (tome)
        let product_symbol = product.unit.dimension().symbol();
        assert!(
            product_symbol.contains("T"),
            "Expected dimension symbol to contain 'T', got: {}",
            product_symbol
        );

        // 5. 测试除法：6 卷 / 2 卷每千克 = 3 千克
        // Test division: 6 T / 2 T/kg = 3 kg
        let six_tome = Quantity::new(BigDecimal::from(6), tome_unit);
        let two_tome_per_kg = Quantity::new(BigDecimal::from(2), tome_per_kilogram);
        let result = &six_tome / &two_tome_per_kg;

        // 验证结果值为 3
        // Verify result value is 3
        assert_eq!(result.value, BigDecimal::from(3));
        // 验证结果量纲包含 M（质量）
        // Verify result dimension contains M (mass)
        let result_symbol = result.unit.dimension().symbol();
        assert!(
            result_symbol.contains("M"),
            "Expected dimension symbol to contain 'M', got: {}",
            result_symbol
        );
    }

    // ========================================================================
    // 测试：与预定义单位混合运算 / Test: Mixed operations with predefined units
    // ========================================================================

    #[test]
    fn test_mixed_units_with_predefined() {
        use crate::unit::CTUnit;
        use crate::unit::derived::Kilogram;

        // 定义"卷"量纲（使用 CustomFundamentalDimension）
        // Define "Tome" dimension (using CustomFundamentalDimension)
        let tome = CustomFundamentalDimension::new("T");
        let tome_dimension = tome.to_enum();
        let tome_quantity = create_custom_dimension(tome_dimension.clone(), "tome");

        // 定义"卷"单位
        // Define "Tome" unit
        let tome_unit = create_custom_unit("tome", "T", tome_quantity.clone(), Scale::new());

        // 定义"卷每千克"单位（使用预定义的千克）
        // Define "Tome per kilogram" unit (using predefined kilogram)
        let mass_quantity =
            DerivedQuantity::from_base("mass".to_string(), FundamentalQuantityEnum::Mass);
        let tome_per_kg_quantity = (&tome_quantity / &mass_quantity).build();
        let tome_per_kilogram = create_custom_unit(
            "tome per kilogram",
            "T/kg",
            tome_per_kg_quantity,
            Scale::new(),
        );

        // 测试：使用预定义千克单位和自定义卷每千克单位
        // Test: Using predefined kilogram unit and custom tome per kilogram unit
        let three_kg = Quantity::new(BigDecimal::from(3), Kilogram::INSTANT.clone());
        let three_tome_per_kg = Quantity::new(BigDecimal::from(3), tome_per_kilogram);
        let product = &three_kg * &three_tome_per_kg;

        // 验证结果值
        // Verify result value
        assert_eq!(product.value, BigDecimal::from(9));
        // 验证结果量纲包含 T（卷）
        // Verify result dimension contains T (tome)
        assert!(product.unit.dimension().symbol().contains("T"));
    }
}
