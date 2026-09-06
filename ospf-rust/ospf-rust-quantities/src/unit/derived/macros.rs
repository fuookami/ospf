//! 单位定义宏 / Unit definition macros
//!
//! 提供用于定义 SI 单位的宏，包括基本单位定义宏和导出单位定义宏 / Provides macros for defining SI units, including base unit definition macros and derived unit definition macros

/// 定义 SI 单位（运行时 + 编译时）
/// Define SI unit (runtime + compile-time)
///
/// # 参数 / Parameters
/// - `$name`: 单位名称标识符（如 METER） / Unit name identifier (e.g., METER)
/// - `$full_name`: 完整名称（如 "meter"） / Full name (e.g., "meter")
/// - `$symbollic`: 符号（如 "m"） / Symbol (e.g., "m")
/// - `$dim`: 量纲类型（如 Length，来自 dimension::derived） / Dimension type (e.g., Length, from dimension::derived)
/// - `$(, scale = $scale:expr)?`: 可选的缩放因子 / Optional scale factor
///
/// # 生成 / Generates
/// - `$name`: 编译时单位结构体（同时实现 CTUnit 和 UnitTrait）/ Compile-time unit struct (implementing both CTUnit and UnitTrait)
#[macro_export]
macro_rules! define_unit {
    // 无缩放参数
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty) => {
        define_unit!(
            $name,
            $full_name,
            $symbol,
            $dim,
            $crate::scale::Scale::new()
        );
    };
    // 带缩放参数
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty, $scale:expr) => {
        /// 编译时单位 / Compile-time unit
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl $crate::unit::physical_unit::CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: once_cell::sync::Lazy<$crate::scale::Scale> =
                once_cell::sync::Lazy::new(|| $scale);
            type Dimension = $dim;
        }

        impl $crate::unit::concept::UnitTrait for $name {
            type Dimension = $dim;

            fn symbol(&self) -> &'static str {
                $symbol
            }

            fn name(&self) -> &'static str {
                $full_name
            }

            fn dimension_symbol(&self) -> String {
                <$dim as $crate::dimension::derived_quantity::CTDerivedQuantity>::INSTANT
                    .symbol()
                    .to_string()
            }

            fn scale_value(&self) -> bigdecimal::BigDecimal {
                Self::SCALE.value().clone()
            }
        }
    };
    // 带缩放和 offset 参数 / With scale and offset parameters
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty, $scale:expr, offset = $offset:expr) => {
        /// 编译时单位 / Compile-time unit
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl $crate::unit::physical_unit::CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: once_cell::sync::Lazy<$crate::scale::Scale> =
                once_cell::sync::Lazy::new(|| $scale);
            const OFFSET: once_cell::sync::Lazy<bigdecimal::BigDecimal> =
                once_cell::sync::Lazy::new(|| $offset);
            type Dimension = $dim;
        }

        impl $crate::unit::concept::UnitTrait for $name {
            type Dimension = $dim;

            fn symbol(&self) -> &'static str {
                $symbol
            }

            fn name(&self) -> &'static str {
                $full_name
            }

            fn dimension_symbol(&self) -> String {
                <$dim as $crate::dimension::derived_quantity::CTDerivedQuantity>::INSTANT
                    .symbol()
                    .to_string()
            }

            fn scale_value(&self) -> bigdecimal::BigDecimal {
                Self::SCALE.value().clone()
            }
        }
    };
    // 带缩放和 domain 参数 / With scale and domain parameters
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty, $scale:expr, domain = $domain:expr) => {
        /// 编译时单位 / Compile-time unit
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl $crate::unit::physical_unit::CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: once_cell::sync::Lazy<$crate::scale::Scale> =
                once_cell::sync::Lazy::new(|| $scale);
            const DOMAIN: $crate::dimension::derived_quantity::QuantityDomain = $domain;
            type Dimension = $dim;
        }

        impl $crate::unit::concept::UnitTrait for $name {
            type Dimension = $dim;

            fn symbol(&self) -> &'static str {
                $symbol
            }

            fn name(&self) -> &'static str {
                $full_name
            }

            fn dimension_symbol(&self) -> String {
                <$dim as $crate::dimension::derived_quantity::CTDerivedQuantity>::INSTANT
                    .symbol()
                    .to_string()
            }

            fn scale_value(&self) -> bigdecimal::BigDecimal {
                Self::SCALE.value().clone()
            }
        }
    };
}

#[macro_export]
macro_rules! define_unit_by {
    // 单位计算
    ($name:ident, $full_name:expr, $symbol:expr, $ct_expr:ty) => {
        /// 编译时单位 / Compile-time unit
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl $crate::unit::physical_unit::CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: once_cell::sync::Lazy<$crate::scale::Scale> = once_cell::sync::Lazy::new(|| <$ct_expr as $crate::unit::physical_unit::CTUnit>::SCALE.clone());
            const OFFSET: once_cell::sync::Lazy<bigdecimal::BigDecimal> = once_cell::sync::Lazy::new(|| <$ct_expr as $crate::unit::physical_unit::CTUnit>::OFFSET.clone());
            const DOMAIN: $crate::dimension::derived_quantity::QuantityDomain = <$ct_expr as $crate::unit::physical_unit::CTUnit>::DOMAIN;
            type Dimension = <$ct_expr as $crate::unit::physical_unit::CTUnit>::Dimension;
        }

        impl $crate::unit::concept::UnitTrait for $name {
            type Dimension = <$ct_expr as $crate::unit::physical_unit::CTUnit>::Dimension;

            fn symbol(&self) -> &'static str {
                $symbol
            }

            fn name(&self) -> &'static str {
                $full_name
            }

            fn dimension_symbol(&self) -> String {
                <<$ct_expr as $crate::unit::physical_unit::CTUnit>::Dimension as $crate::dimension::derived_quantity::CTDerivedQuantity>::INSTANT.symbol().to_string()
            }

            fn scale_value(&self) -> bigdecimal::BigDecimal {
                Self::SCALE.value().clone()
            }
        }
    };
}
