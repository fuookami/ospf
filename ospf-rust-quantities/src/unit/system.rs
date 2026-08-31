//! Unit system - 单位制
//! Unit system - Unit systems (SI, MKS, CGS, etc.)
//!
//! 单位制定义了基本单位，导出单位通过懒加载自动推导。
//! Unit systems define base units, derived units are lazily computed.
//!
//! # 预定义单位制 / Predefined Unit Systems
//! - `SI_SYSTEM`: 国际单位制（米、千克、秒、安培、开尔文、摩尔、坎德拉）
//! - `MKS_SYSTEM`: 米-千克-秒单位制
//! - `CGS_SYSTEM`: 厘米-克-秒单位制
//!
//! # 自定义单位制 / Custom Unit Systems
//! 使用 `UnitSystemBuilder` 可以创建自定义单位制：
//! - `new()`: 创建空单位制
//! - `from_prototype()`: 从现有单位制继承
//! - `with_base_unit()`: 添加基本单位
//! - `with_derived_unit()`: 添加导出单位

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ospf_rust_base::read_unwrap;
use ospf_rust_base::write_unwrap;
use crate::dimension::derived_quantity::DerivedQuantity;
use crate::dimension::fundamental_quantity::FundamentalQuantityEnum;
use crate::scale::Scale;
use crate::unit::physical_unit::Unit;

// ============================================================================
// UnitSystem trait - 单位制 trait
// ============================================================================

/// UnitSystem - 单位制 trait
/// UnitSystem - Unit system trait
pub trait UnitSystem: std::fmt::Debug + Send + Sync + 'static {
    /// 单位制名称
    /// Unit system name
    fn name(&self) -> &str;

    /// 获取基本单位
    /// Get base units
    fn base_units(&self) -> &HashMap<FundamentalQuantityEnum, Unit>;

    /// 获取用户指定的标准单位
    /// Get user-specified standard units
    fn standard_units(&self) -> &RwLock<HashMap<DerivedQuantity, Unit>>;

    /// 获取指定量纲的标准单位
    /// Get standard unit for dimension
    ///
    /// 如果用户指定了标准单位，返回用户指定的；否则返回推导的默认单位
    /// Returns user-specified standard unit if set, otherwise returns derived default unit
    fn standard_unit_for_dimension(&self, dimension: &DerivedQuantity) -> Option<Unit> {
        // 1. 首先检查用户是否指定了标准单位
        // First check if user has specified a standard unit
        {
            let cache = read_unwrap!(self.standard_units());
            if let Some(unit) = cache.get(dimension) {
                return Some(unit.clone());
            }
        }

        // 2. 否则使用推导的默认单位
        // Otherwise use derived default unit
        self.unit_for_dimension(dimension)
    }

    /// 设置指定量纲的标准单位
    /// Set standard unit for dimension
    ///
    /// 允许在单位制创建后动态修改标准单位
    /// Allows dynamic modification of standard units after unit system creation
    fn set_standard_unit(&self, dimension: DerivedQuantity, unit: Unit) {
        let mut cache = write_unwrap!(self.standard_units());
        cache.insert(dimension, unit);
    }

    /// 移除指定量纲的标准单位（恢复使用默认推导单位）
    /// Remove standard unit for dimension (revert to default derived unit)
    fn remove_standard_unit(&self, dimension: &DerivedQuantity) -> bool {
        let mut cache = write_unwrap!(self.standard_units());
        cache.remove(dimension).is_some()
    }

    /// 获取指定量纲的单位（懒加载推导）
    /// Get unit for dimension (lazy derivation)
    fn unit_for_dimension(&self, dimension: &DerivedQuantity) -> Option<Unit> {
        // 1. 检查是否是基本量纲
        // Check if it's a fundamental dimension
        if dimension.powers().count() == 1 {
            if let Some(dp) = dimension.powers().next() {
                if dp.power == 1 {
                    // 尝试转换为基本量纲
                    // Try to convert to fundamental dimension
                    if let Some(fund_dim) = dimension_to_fundamental(&dp.dimension) {
                        return self.base_units().get(&fund_dim).cloned();
                    }
                }
            }
        }

        // 2. 检查缓存
        // Check cache
        {
            let cache = read_unwrap!(self.derived_cache());
            if let Some(unit) = cache.get(dimension) {
                return Some(unit.clone());
            }
        }

        // 3. 推导单位
        // Derive unit
        let unit = self.derive_unit(dimension)?;

        // 4. 存入缓存
        // Store in cache
        {
            let mut cache = write_unwrap!(self.derived_cache());
            cache.insert(dimension.clone(), unit.clone());
        }

        Some(unit)
    }

    /// 获取导出单位缓存
    /// Get derived units cache
    fn derived_cache(&self) -> &RwLock<HashMap<DerivedQuantity, Unit>>;

    /// 推导指定量纲的单位
    /// Derive unit for dimension
    fn derive_unit(&self, dimension: &DerivedQuantity) -> Option<Unit> {
        let mut result_scale = Scale::new();
        let mut result_dimension = DerivedQuantity::none(String::new());
        let mut symbol_parts: Vec<String> = Vec::new();

        for dp in dimension.powers() {
            let fund_dim = dimension_to_fundamental(&dp.dimension)?;
            let base_unit = self.base_units().get(&fund_dim)?;

            let power = dp.power;
            if power != 0 {
                // 累积比例
                // Accumulate scale
                let unit_scale = base_unit.scale().clone();
                if power == 1 {
                    result_scale = result_scale * unit_scale;
                } else if power == -1 {
                    result_scale = result_scale / unit_scale;
                } else {
                    // 处理非整数幂
                    // Handle non-integer powers
                    let pow_scale = unit_scale.pow(&bigdecimal::BigDecimal::from(power));
                    result_scale = result_scale * pow_scale;
                }

                // 累积量纲
                // Accumulate dimension
                result_dimension = (result_dimension
                    * DerivedQuantity::from_base_with_power(
                        String::new(),
                        fund_dim.clone(),
                        power,
                    ))
                .into();

                // 构建符号
                // Build symbol
                let unit_sym = base_unit.symbol().to_string();
                if power == 1 {
                    symbol_parts.push(unit_sym);
                } else if power == -1 {
                    symbol_parts.push(format!("{}/", unit_sym));
                } else {
                    symbol_parts.push(format!("{}^{}", unit_sym, power));
                }
            }
        }

        let symbol = symbol_parts.join("·");
        let name = format!("derived_{}", dimension.symbol());

        Some(Unit::new(name, symbol, dimension.clone(), result_scale))
    }

    /// 获取指定量纲相对于标准单位制的比例
    /// Get conversion scale to standard system for dimension
    fn conversion_to_standard(&self, dimension: &DerivedQuantity) -> Option<Scale> {
        let unit = self.unit_for_dimension(dimension)?;
        Some(unit.scale().clone())
    }
}

/// 将 DimensionPower 中的 dimension 转换为基本量纲枚举
/// Convert dimension in DimensionPower to fundamental dimension enum
fn dimension_to_fundamental(dim: &FundamentalQuantityEnum) -> Option<FundamentalQuantityEnum> {
    Some(dim.clone())
}

// ============================================================================
// ConcreteUnitSystem - 具体单位制实现
// ============================================================================

/// 具体单位制实现 / Concrete unit system implementation
#[derive(Debug)]
pub struct ConcreteUnitSystem {
    name: String,
    base_units: HashMap<FundamentalQuantityEnum, Unit>,
    derived_cache: RwLock<HashMap<DerivedQuantity, Unit>>,
    /// 用户指定的标准单位
    /// User-specified standard units
    standard_units: RwLock<HashMap<DerivedQuantity, Unit>>,
}

impl ConcreteUnitSystem {
    /// 创建新的单位制
    /// Create new unit system
    pub fn new(name: &str, base_units: HashMap<FundamentalQuantityEnum, Unit>) -> Self {
        Self {
            name: name.to_string(),
            base_units,
            derived_cache: RwLock::new(HashMap::new()),
            standard_units: RwLock::new(HashMap::new()),
        }
    }

    /// 创建带标准单位的新单位制
    /// Create new unit system with standard units
    pub fn with_standard_units(
        name: &str,
        base_units: HashMap<FundamentalQuantityEnum, Unit>,
        standard_units: HashMap<DerivedQuantity, Unit>,
    ) -> Self {
        Self {
            name: name.to_string(),
            base_units,
            derived_cache: RwLock::new(HashMap::new()),
            standard_units: RwLock::new(standard_units),
        }
    }
}

impl UnitSystem for ConcreteUnitSystem {
    fn name(&self) -> &str {
        &self.name
    }

    fn base_units(&self) -> &HashMap<FundamentalQuantityEnum, Unit> {
        &self.base_units
    }

    fn derived_cache(&self) -> &RwLock<HashMap<DerivedQuantity, Unit>> {
        &self.derived_cache
    }

    fn standard_units(&self) -> &RwLock<HashMap<DerivedQuantity, Unit>> {
        &self.standard_units
    }
}

// ============================================================================
// UnitSystemBuilder - 单位制构建器
// ============================================================================

/// 单位制构建器 / Unit system builder
#[derive(Debug)]
pub struct UnitSystemBuilder {
    name: String,
    prototype: Option<Arc<dyn UnitSystem>>,
    base_units: HashMap<FundamentalQuantityEnum, Unit>,
    derived_units: HashMap<DerivedQuantity, Unit>,
    /// 用户指定的标准单位
    /// User-specified standard units
    standard_units: HashMap<DerivedQuantity, Unit>,
}

impl UnitSystemBuilder {
    /// 创建新的单位制构建器
    /// Create new unit system builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            prototype: None,
            base_units: HashMap::new(),
            derived_units: HashMap::new(),
            standard_units: HashMap::new(),
        }
    }

    /// 从原型单位制创建（继承其所有单位）
    /// Create from prototype unit system (inherit all units)
    pub fn from_prototype(name: &str, prototype: Arc<dyn UnitSystem>) -> Self {
        Self {
            name: name.to_string(),
            prototype: Some(prototype.clone()),
            base_units: prototype.base_units().clone(),
            derived_units: HashMap::new(),
            standard_units: {
                let cache = read_unwrap!(prototype.standard_units());
                cache.clone()
            },
        }
    }

    /// 添加/替换基本单位
    /// Add/replace base unit
    pub fn with_base_unit(mut self, dimension: FundamentalQuantityEnum, unit: Unit) -> Self {
        self.base_units.insert(dimension, unit);
        self
    }

    /// 添加/替换导出单位
    /// Add/replace derived unit
    pub fn with_derived_unit(mut self, dimension: DerivedQuantity, unit: Unit) -> Self {
        self.derived_units.insert(dimension, unit);
        self
    }

    /// 设置指定量纲的标准单位
    /// Set standard unit for dimension
    ///
    /// 标准单位用于将物理量转换为该量纲的标准表示
    /// Standard units are used to convert quantities to standard representation for that dimension
    pub fn with_standard_unit(mut self, dimension: DerivedQuantity, unit: Unit) -> Self {
        self.standard_units.insert(dimension, unit);
        self
    }

    /// 构建单位制
    /// Build unit system
    pub fn build(self) -> Arc<dyn UnitSystem> {
        let system = Arc::new(ConcreteUnitSystem::with_standard_units(
            &self.name,
            self.base_units,
            self.standard_units,
        ));

        // 预存导出单位到缓存
        // Pre-store derived units to cache
        {
            let mut cache = write_unwrap!(system.derived_cache());
            for (dim, unit) in self.derived_units {
                cache.insert(dim, unit);
            }
        }

        system
    }
}

// ============================================================================
// SI 基本单位（从 derived 模块重导出）/ SI base units (re-exported from derived module)
// ============================================================================

use once_cell::sync::Lazy;

use crate::unit::derived::{
    Ampere, Bit, Candela, Cetimeter, Gram, Kelvin, Kilogram, Meter, Mole, Radian, Second, Steradian,
};
// 从各子模块导入基本单位类型 / Import base unit types from submodules
use crate::unit::physical_unit::CTUnit;

// ============================================================================
// SI 单位制 / SI unit system
// ============================================================================

/// SI单位制 / SI unit system
pub static SI_SYSTEM: Lazy<Arc<dyn UnitSystem>> = Lazy::new(|| {
    let mut base_units: HashMap<FundamentalQuantityEnum, Unit> = HashMap::new();
    base_units.insert(FundamentalQuantityEnum::Length, Meter::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Time, Second::INSTANT.clone());
    base_units.insert(
        FundamentalQuantityEnum::ElectricCurrent,
        Ampere::INSTANT.clone(),
    );
    base_units.insert(
        FundamentalQuantityEnum::ThermodynamicTemperature,
        Kelvin::INSTANT.clone(),
    );
    base_units.insert(
        FundamentalQuantityEnum::AmountOfSubstance,
        Mole::INSTANT.clone(),
    );
    base_units.insert(
        FundamentalQuantityEnum::LuminousIntensity,
        Candela::INSTANT.clone(),
    );
    base_units.insert(FundamentalQuantityEnum::Information, Bit::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::PlaneAngle, Radian::INSTANT.clone());
    base_units.insert(
        FundamentalQuantityEnum::SolidAngle,
        Steradian::INSTANT.clone(),
    );

    Arc::new(ConcreteUnitSystem::new("SI", base_units))
});

// ============================================================================
// MKS 单位制 / MKS unit system
// ============================================================================

/// MKS单位制（米-千克-秒）/ MKS unit system (meter-kilogram-second)
pub static MKS_SYSTEM: Lazy<Arc<dyn UnitSystem>> = Lazy::new(|| {
    let mut base_units: HashMap<FundamentalQuantityEnum, Unit> = HashMap::new();
    base_units.insert(FundamentalQuantityEnum::Length, Meter::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Time, Second::INSTANT.clone());

    Arc::new(ConcreteUnitSystem::new("MKS", base_units))
});

// ============================================================================
// CGS 单位制 / CGS unit system
// ============================================================================

/// CGS单位制（厘米-克-秒）/ CGS unit system (centimeter-gram-second)
pub static CGS_SYSTEM: Lazy<Arc<dyn UnitSystem>> = Lazy::new(|| {
    let mut base_units: HashMap<FundamentalQuantityEnum, Unit> = HashMap::new();
    base_units.insert(FundamentalQuantityEnum::Length, Cetimeter::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Mass, Gram::INSTANT.clone());
    base_units.insert(FundamentalQuantityEnum::Time, Second::INSTANT.clone());

    Arc::new(ConcreteUnitSystem::new("CGS", base_units))
});

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::*;

    #[test]
    fn test_si_system() {
        assert_eq!(SI_SYSTEM.name(), "SI");
    }

    #[test]
    fn test_si_base_units() {
        let meter = Meter::INSTANT.clone();
        assert_eq!(meter.name(), "meter");
        assert_eq!(meter.symbol(), "m");
        assert_eq!(meter.scale().value(), &BigDecimal::from(1));

        let kg = Kilogram::INSTANT.clone();
        assert_eq!(kg.name(), "kilogram");
        let sec = Second::INSTANT.clone();
        assert_eq!(sec.name(), "second");
    }

    #[test]
    fn test_si_base_unit_dimensions() {
        let meter = Meter::INSTANT.clone();
        let kg = Kilogram::INSTANT.clone();
        let sec = Second::INSTANT.clone();
        assert_eq!(meter.dimension().symbol(), "L");
        assert_eq!(kg.dimension().symbol(), "M");
        assert_eq!(sec.dimension().symbol(), "T");
    }

    #[test]
    fn test_mks_system() {
        assert_eq!(MKS_SYSTEM.name(), "MKS");

        // 验证基本单位
        // Verify base units
        let length_unit = MKS_SYSTEM
            .base_units()
            .get(&FundamentalQuantityEnum::Length);
        assert!(length_unit.is_some());
        assert_eq!(length_unit.unwrap().symbol(), "m");
    }

    #[test]
    fn test_cgs_system() {
        assert_eq!(CGS_SYSTEM.name(), "CGS");

        // 验证基本单位
        // Verify base units
        let length_unit = CGS_SYSTEM
            .base_units()
            .get(&FundamentalQuantityEnum::Length);
        assert!(length_unit.is_some());
        assert_eq!(length_unit.unwrap().symbol(), "cm");

        let mass_unit = CGS_SYSTEM.base_units().get(&FundamentalQuantityEnum::Mass);
        assert!(mass_unit.is_some());
        assert_eq!(mass_unit.unwrap().symbol(), "g");
    }

    #[test]
    fn test_unit_system_builder() {
        // 使用构建器创建自定义单位制
        // Create custom unit system using builder
        let custom = UnitSystemBuilder::new("Custom")
            .with_base_unit(FundamentalQuantityEnum::Length, Meter::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Time, Second::INSTANT.clone())
            .build();

        assert_eq!(custom.name(), "Custom");
        assert!(
            custom
                .base_units()
                .contains_key(&FundamentalQuantityEnum::Length)
        );
    }

    #[test]
    fn test_unit_system_builder_from_prototype() {
        // 从原型创建
        // Create from prototype
        let custom = UnitSystemBuilder::from_prototype("CustomSI", SI_SYSTEM.clone()).build();

        assert_eq!(custom.name(), "CustomSI");
        // 应该继承 SI 的所有基本单位
        // Should inherit all base units from SI
        assert!(
            custom
                .base_units()
                .contains_key(&FundamentalQuantityEnum::Length)
        );
        assert!(
            custom
                .base_units()
                .contains_key(&FundamentalQuantityEnum::Mass)
        );
    }

    #[test]
    fn test_lazy_derived_unit() {
        // 测试懒加载推导单位
        // Test lazy derived unit
        let length_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);
        let time_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Time);
        let velocity_dim = (length_dim / time_dim).into();

        let velocity_unit = MKS_SYSTEM.unit_for_dimension(&velocity_dim);
        assert!(velocity_unit.is_some());

        let unit = velocity_unit.unwrap();
        assert_eq!(unit.symbol(), "m·s/");
        assert_eq!(*unit.dimension(), velocity_dim);
    }

    #[test]
    fn test_cgs_derived_unit() {
        // 测试 CGS 系统的导出单位
        // Test derived units in CGS system
        let length_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);
        let time_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Time);
        let velocity_dim = (length_dim / time_dim).into();

        let velocity_unit = CGS_SYSTEM.unit_for_dimension(&velocity_dim);
        assert!(velocity_unit.is_some());

        let unit = velocity_unit.unwrap();
        assert_eq!(unit.symbol(), "cm·s/");
    }

    #[test]
    fn test_standard_unit_default() {
        // 测试默认标准单位（未指定时使用推导单位）
        // Test default standard unit (uses derived unit when not specified)
        let length_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);

        // SI 系统中长度的默认标准单位应该是米
        // Default standard unit for length in SI should be meter
        let standard_unit = SI_SYSTEM.standard_unit_for_dimension(&length_dim);
        assert!(standard_unit.is_some());
        assert_eq!(standard_unit.unwrap().symbol(), "m");
    }

    #[test]
    fn test_standard_unit_custom() {
        // 测试自定义标准单位
        // Test custom standard unit
        use crate::unit::CTUnit;
        use crate::unit::Kilometer;

        let length_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);

        // 创建自定义单位制，指定长度的标准单位为千米
        // Create custom unit system with kilometer as standard unit for length
        let custom_system = UnitSystemBuilder::new("CustomLength")
            .with_base_unit(FundamentalQuantityEnum::Length, Meter::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Time, Second::INSTANT.clone())
            .with_standard_unit(length_dim.clone(), Kilometer::INSTANT.clone())
            .build();

        // 长度的标准单位应该是千米
        // Standard unit for length should be kilometer
        let standard_unit = custom_system.standard_unit_for_dimension(&length_dim);
        assert!(standard_unit.is_some());
        assert_eq!(standard_unit.unwrap().symbol(), "km");
    }

    #[test]
    fn test_quantity_to_standard_unit() {
        // 测试物理量转换为标准单位
        // Test quantity conversion to standard unit
        use crate::quantity::Quantity;
        use crate::unit::{CTUnit, Kilometer};
        use bigdecimal::BigDecimal;

        // 使用 Meter::INSTANT 的量纲作为标准单位的量纲
        // Use the dimension from Meter::INSTANT for the standard unit dimension
        let length_dim = Meter::INSTANT.dimension().clone();

        // 创建自定义单位制，指定长度的标准单位为千米
        // Create custom unit system with kilometer as standard unit for length
        let custom_system = UnitSystemBuilder::new("CustomLength")
            .with_base_unit(FundamentalQuantityEnum::Length, Meter::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Time, Second::INSTANT.clone())
            .with_standard_unit(length_dim.clone(), Kilometer::INSTANT.clone())
            .build();

        // 创建一个以米为单位的物理量
        // Create a quantity in meters
        let quantity = Quantity::new(BigDecimal::from(1000), Meter::INSTANT.clone());

        // 转换为标准单位（应该是千米）
        // Convert to standard unit (should be kilometer)
        let standard_quantity = quantity.to_standard_unit(custom_system.as_ref());
        assert!(standard_quantity.is_some());

        let sq = standard_quantity.unwrap();
        assert_eq!(sq.unit.symbol(), "km");
        assert_eq!(sq.value, BigDecimal::from(1));
    }

    #[test]
    fn test_set_standard_unit_runtime() {
        // 测试运行时设置标准单位
        // Test setting standard unit at runtime
        use crate::unit::CTUnit;
        use crate::unit::Kilometer;

        let length_dim = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);

        // 创建单位制（未指定标准单位）
        // Create unit system without standard unit
        let custom_system = UnitSystemBuilder::new("RuntimeStandard")
            .with_base_unit(FundamentalQuantityEnum::Length, Meter::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Mass, Kilogram::INSTANT.clone())
            .with_base_unit(FundamentalQuantityEnum::Time, Second::INSTANT.clone())
            .build();

        // 默认标准单位是米
        // Default standard unit is meter
        let default_unit = custom_system.standard_unit_for_dimension(&length_dim);
        assert!(default_unit.is_some());
        assert_eq!(default_unit.unwrap().symbol(), "m");

        // 运行时设置标准单位为千米
        // Set standard unit to kilometer at runtime
        custom_system.set_standard_unit(length_dim.clone(), Kilometer::INSTANT.clone());

        // 现在标准单位应该是千米
        // Now standard unit should be kilometer
        let new_standard = custom_system.standard_unit_for_dimension(&length_dim);
        assert!(new_standard.is_some());
        assert_eq!(new_standard.unwrap().symbol(), "km");

        // 移除标准单位，恢复默认
        // Remove standard unit, revert to default
        let removed = custom_system.remove_standard_unit(&length_dim);
        assert!(removed);

        // 恢复后的标准单位应该是米
        // Reverted standard unit should be meter
        let reverted = custom_system.standard_unit_for_dimension(&length_dim);
        assert!(reverted.is_some());
        assert_eq!(reverted.unwrap().symbol(), "m");
    }
}
