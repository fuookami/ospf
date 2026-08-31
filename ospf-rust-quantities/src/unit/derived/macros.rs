//! Unit definition macros - 单位定义宏
//! Unit definition macros - Macros for defining SI units
//!
//! 提供用于定义 SI 单位的宏，包括基本单位定义宏和导出单位定义宏。
//! Provides macros for defining SI units, including base unit definition macros and derived unit definition macros.

/// 定义 SI 单位（运行时 + 编译时）
/// Define SI unit (runtime + compile-time)
///
/// # 参数 / Parameters
/// - `$name`: 单位名称标识符（如 METER） / Unit name identifier (e.g., METER)
/// - `$full_name`: 完整名称（如 "meter"） / Full name (e.g., "meter")
/// - `$symbol`: 符号（如 "m"） / Symbol (e.g., "m")
/// - `$dim`: 量纲类型（如 Length，来自 dimension::derived） / Dimension type (e.g., Length, from dimension::derived)
/// - `$(, scale = $scale:expr)?`: 可选的缩放因子 / Optional scale factor
///
/// # 生成 / Generates
/// - `$name`: 运行时单位静态变量 / Runtime unit static variable
/// - `CTUnit$name`: 编译时单位结构体 / Compile-time unit struct
#[macro_export]
macro_rules! define_unit {
    // 无缩放参数
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty) => {
        define_unit!($name, $full_name, $symbol, $dim, Scale::new());
    };
    // 带缩放参数
    ($name:ident, $full_name:expr, $symbol:expr, $dim:ty, $scale:expr) => {
        /// 编译时单位 / Compile-time unit
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: Lazy<Scale> = Lazy::new(|| $scale);
            type Dimension = $dim;
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

        impl CTUnit for $name {
            const NAME: &'static str = $full_name;
            const SYMBOL: &'static str = $symbol;
            const SCALE: Lazy<Scale> = Lazy::new(|| <$ct_expr as CTUnit>::SCALE.clone());
            type Dimension = <$ct_expr as CTUnit>::Dimension;
        }
    };
}
