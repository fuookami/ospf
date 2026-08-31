// ============================================================================
// ID 值类型 / ID value types
//
// 为 CSP1D 领域实体提供强类型字符串 ID，包括物料、产品、配规、设备和切割方案。
// Provides strongly-typed string IDs for CSP1D domain entities,
// including material, product, costar, machine, and cutting plan.
// ============================================================================

macro_rules! string_id_type {
    ($name:ident, $doc_zh:literal, $doc_en:literal) => {
        #[doc = concat!($doc_zh, " / ", $doc_en)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// 创建 ID / Create id
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// 字符串视图 / String view
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 转为字符串 / Convert into string
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

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
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

string_id_type!(MaterialId, "物料 ID", "Material id");
string_id_type!(ProductId, "产品 ID", "Product id");
string_id_type!(CostarId, "配规 ID", "Costar id");
string_id_type!(MachineId, "设备 ID", "Machine id");
string_id_type!(CuttingPlanId, "切割方案 ID", "Cutting plan id");

/// 创建物料 ID / Create material id
pub fn material_id(value: impl Into<String>) -> MaterialId {
    MaterialId::new(value)
}

/// 创建产品 ID / Create product id
pub fn product_id(value: impl Into<String>) -> ProductId {
    ProductId::new(value)
}

/// 创建配规 ID / Create costar id
pub fn costar_id(value: impl Into<String>) -> CostarId {
    CostarId::new(value)
}

/// 创建设备 ID / Create machine id
pub fn machine_id(value: impl Into<String>) -> MachineId {
    MachineId::new(value)
}

/// 创建切割方案 ID / Create cutting plan id
pub fn cutting_plan_id(value: impl Into<String>) -> CuttingPlanId {
    CuttingPlanId::new(value)
}
