// ============================================================================
// ID 值类型 / ID value types
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

string_id_type!(ItemId, "货物 ID", "Item id");
string_id_type!(BinTypeId, "箱型 ID", "Bin type id");

/// 创建货物 ID / Create item id
pub fn item_id(value: impl Into<String>) -> ItemId {
    ItemId::new(value)
}

/// 创建箱型 ID / Create bin type id
pub fn bin_type_id(value: impl Into<String>) -> BinTypeId {
    BinTypeId::new(value)
}
