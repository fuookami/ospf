//! 二维装箱标识类型 / 2D packing identifier types
//!
//! 与 BPP3D 的标识类型对齐：板材类型编码、物品编号、批次号与物料编号。
//! Mirrors the BPP3D identifier types: sheet type codes, item identifiers, batch numbers and material
//! numbers.

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

string_id_type!(SheetTypeId, "板材类型编码", "Sheet type code");
string_id_type!(ItemId, "物品编号", "Item identifier");
string_id_type!(MaterialNo, "物料编号", "Material number");
string_id_type!(BatchNo, "批次号", "Batch number");

/// 创建板材类型编码 / Create a sheet type code
pub fn sheet_type_id(value: impl Into<String>) -> SheetTypeId {
    SheetTypeId::new(value)
}

/// 创建物品编号 / Create an item identifier
pub fn item_id(value: impl Into<String>) -> ItemId {
    ItemId::new(value)
}

/// 创建物料编号 / Create a material number
pub fn material_no(value: impl Into<String>) -> MaterialNo {
    MaterialNo::new(value)
}

/// 创建批次号 / Create a batch number
pub fn batch_no(value: impl Into<String>) -> BatchNo {
    BatchNo::new(value)
}

/// 单批次号标记：表示不区分批次的场景 / Single-batch marker used when batches are not distinguished
pub fn single_batch_no() -> BatchNo {
    BatchNo::new("single")
}
