//! 三值逻辑
//! Trivalent logic

/// 普通三值逻辑：真、假、未知。
/// Ordinary trivalent logic: true, false, and unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Trivalent {
    /// 真 / True
    True,
    /// 假 / False
    False,
    /// 未知 / Unknown
    #[default]
    Unknown,
}

impl Trivalent {
    /// 转换为可选布尔值。
    /// Convert to an optional boolean value.
    pub fn is_true(self) -> Option<bool> {
        self.into()
    }

    /// 获取数值表示。
    /// Get the numeric representation.
    pub fn value(self) -> f64 {
        self.value_f64()
    }

    /// 转换为普通三值数值。
    /// Convert to an ordinary trivalent numeric value.
    pub fn value_f64(self) -> f64 {
        match self {
            Self::True => 1.0,
            Self::False => 0.0,
            Self::Unknown => 0.5,
        }
    }
}

impl From<bool> for Trivalent {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

impl From<Option<bool>> for Trivalent {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::True,
            Some(false) => Self::False,
            None => Self::Unknown,
        }
    }
}

impl From<Trivalent> for Option<bool> {
    fn from(value: Trivalent) -> Self {
        match value {
            Trivalent::True => Some(true),
            Trivalent::False => Some(false),
            Trivalent::Unknown => None,
        }
    }
}

/// 平衡三值逻辑：真、假、未知。
/// Balanced trivalent logic: true, false, and unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BalancedTrivalent {
    /// 真 / True
    True,
    /// 假 / False
    False,
    /// 未知 / Unknown
    #[default]
    Unknown,
}

impl BalancedTrivalent {
    /// 转换为可选布尔值。
    /// Convert to an optional boolean value.
    pub fn is_true(self) -> Option<bool> {
        self.into()
    }

    /// 获取数值表示。
    /// Get the numeric representation.
    pub fn value(self) -> i8 {
        self.value_i8()
    }

    /// 转换为平衡三值数值。
    /// Convert to a balanced trivalent numeric value.
    pub fn value_i8(self) -> i8 {
        match self {
            Self::True => 1,
            Self::False => -1,
            Self::Unknown => 0,
        }
    }
}

impl From<bool> for BalancedTrivalent {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

impl From<Option<bool>> for BalancedTrivalent {
    fn from(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::True,
            Some(false) => Self::False,
            None => Self::Unknown,
        }
    }
}

impl From<BalancedTrivalent> for Option<bool> {
    fn from(value: BalancedTrivalent) -> Self {
        match value {
            BalancedTrivalent::True => Some(true),
            BalancedTrivalent::False => Some(false),
            BalancedTrivalent::Unknown => None,
        }
    }
}

impl From<BalancedTrivalent> for Trivalent {
    fn from(value: BalancedTrivalent) -> Self {
        match value {
            BalancedTrivalent::True => Self::True,
            BalancedTrivalent::False => Self::False,
            BalancedTrivalent::Unknown => Self::Unknown,
        }
    }
}

impl From<Trivalent> for BalancedTrivalent {
    fn from(value: Trivalent) -> Self {
        match value {
            Trivalent::True => Self::True,
            Trivalent::False => Self::False,
            Trivalent::Unknown => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BalancedTrivalent, Trivalent};

    #[test]
    fn trivalent_converts_from_bool_and_option() {
        assert_eq!(Trivalent::from(true), Trivalent::True);
        assert_eq!(Trivalent::from(false), Trivalent::False);
        assert_eq!(Trivalent::from(Some(true)), Trivalent::True);
        assert_eq!(Trivalent::from(None), Trivalent::Unknown);

        let value: Option<bool> = Trivalent::Unknown.into();
        assert_eq!(value, None);
    }

    #[test]
    fn balanced_trivalent_converts_from_bool_and_option() {
        assert_eq!(BalancedTrivalent::from(true), BalancedTrivalent::True);
        assert_eq!(BalancedTrivalent::from(false), BalancedTrivalent::False);
        assert_eq!(
            BalancedTrivalent::from(Some(false)),
            BalancedTrivalent::False
        );
        assert_eq!(BalancedTrivalent::from(None), BalancedTrivalent::Unknown);

        let value: Option<bool> = BalancedTrivalent::True.into();
        assert_eq!(value, Some(true));
    }

    #[test]
    fn trivalent_values_match_kotlin_mapping() {
        assert_eq!(Trivalent::True.value_f64(), 1.0);
        assert_eq!(Trivalent::False.value_f64(), 0.0);
        assert_eq!(Trivalent::Unknown.value_f64(), 0.5);
        assert_eq!(Trivalent::Unknown.value(), 0.5);

        assert_eq!(BalancedTrivalent::True.value_i8(), 1);
        assert_eq!(BalancedTrivalent::False.value_i8(), -1);
        assert_eq!(BalancedTrivalent::Unknown.value_i8(), 0);
        assert_eq!(BalancedTrivalent::Unknown.value(), 0);
    }

    #[test]
    fn trivalent_and_balanced_trivalent_convert_each_other() {
        assert_eq!(Trivalent::from(BalancedTrivalent::True), Trivalent::True);
        assert_eq!(
            BalancedTrivalent::from(Trivalent::False),
            BalancedTrivalent::False
        );
        assert_eq!(
            BalancedTrivalent::from(Trivalent::Unknown),
            BalancedTrivalent::Unknown
        );
    }
}
