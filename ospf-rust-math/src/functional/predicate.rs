pub type Predicate<T> = dyn Fn(&T) -> bool;
pub type Comparator<T> = dyn Fn(&T, &T) -> bool;
pub type ComparatorBetween<T, U> = dyn Fn(&T, &U) -> bool;
