pub enum Trivalent {
    True,
    False,
    Unknown,
}

pub type Triv = Trivalent;

impl From<bool> for Trivalent {
    fn from(value: bool) -> Self {
        if value {
            Trivalent::True
        } else {
            Trivalent::False
        }
    }
}

impl From<Option<bool>> for Trivalent {
    fn from(value: Option<bool>) -> Self {
        if let Some(value) = value {
            Trivalent::from(value)
        } else {
            Trivalent::Unknown
        }
    }
}

impl From<BalancedTrivalent> for Trivalent {
    fn from(value: BalancedTrivalent) -> Self {
        match value {
            BalancedTrivalent::True => Trivalent::True,
            BalancedTrivalent::False => Trivalent::False,
            BalancedTrivalent::Unknown => Trivalent::Unknown,
        }
    }
}

impl Trivalent {
    fn is_true(&self) -> Option<bool> {
        match self {
            Trivalent::True => Some(true),
            Trivalent::False => Some(false),
            Trivalent::Unknown => None,
        }
    }
}

pub enum BalancedTrivalent {
    True,
    False,
    Unknown,
}

pub type BTriv = BalancedTrivalent;

impl From<bool> for BalancedTrivalent {
    fn from(value: bool) -> Self {
        if value {
            BalancedTrivalent::True
        } else {
            BalancedTrivalent::False
        }
    }
}

impl From<Option<bool>> for BalancedTrivalent {
    fn from(value: Option<bool>) -> Self {
        if let Some(value) = value {
            BalancedTrivalent::from(value)
        } else {
            BalancedTrivalent::Unknown
        }
    }
}

impl From<Trivalent> for BalancedTrivalent {
    fn from(value: Trivalent) -> Self {
        match value {
            Trivalent::True => BalancedTrivalent::True,
            Trivalent::False => BalancedTrivalent::False,
            Trivalent::Unknown => BalancedTrivalent::Unknown,
        }
    }
}

impl BalancedTrivalent {
    fn is_true(&self) -> bool {
        match self {
            BalancedTrivalent::True => true,
            BalancedTrivalent::False => false,
            BalancedTrivalent::Unknown => false,
        }
    }
}
