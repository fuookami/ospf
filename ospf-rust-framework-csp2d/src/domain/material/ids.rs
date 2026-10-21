//! 二维标识类型 / 2D identifier types
//!
//! 与 Kotlin `Ids2.kt` 对齐：产品、物料、设备、副产物、图案与方案的标识。
//! Mirrors Kotlin `Ids2.kt`: identifiers for products, materials, machines, by-products, patterns and plans.

macro_rules! string_id_type {
    ($name:ident, $doc_zh:literal, $doc_en:literal) => {
        #[doc = concat!($doc_zh, " / ", $doc_en)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// 创建标识 / Create the identifier
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// 字符串视图 / String view
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 转为字符串 / Convert into a string
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }
    };
}

string_id_type!(ProductId2, "产品编号", "Product identifier");
string_id_type!(MaterialId2, "物料编号", "Material identifier");
string_id_type!(MachineId2, "设备编号", "Machine identifier");
string_id_type!(CostarId2, "副产物编号", "By-product identifier");
string_id_type!(PatternId2, "图案编号", "Pattern identifier");
string_id_type!(PlanId2, "方案编号", "Plan identifier");

/// 创建产品编号 / Create a product identifier
pub fn product_id2(value: impl Into<String>) -> ProductId2 {
    ProductId2::new(value)
}

/// 创建物料编号 / Create a material identifier
pub fn material_id2(value: impl Into<String>) -> MaterialId2 {
    MaterialId2::new(value)
}

/// 创建设备编号 / Create a machine identifier
pub fn machine_id2(value: impl Into<String>) -> MachineId2 {
    MachineId2::new(value)
}

/// 创建副产物编号 / Create a by-product identifier
pub fn costar_id2(value: impl Into<String>) -> CostarId2 {
    CostarId2::new(value)
}

/// 创建图案编号 / Create a pattern identifier
pub fn pattern_id2(value: impl Into<String>) -> PatternId2 {
    PatternId2::new(value)
}

/// 创建方案编号 / Create a plan identifier
pub fn plan_id2(value: impl Into<String>) -> PlanId2 {
    PlanId2::new(value)
}
