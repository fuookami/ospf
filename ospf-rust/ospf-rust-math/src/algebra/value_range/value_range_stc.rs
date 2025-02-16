use super::*;
use crate::algebra::*;

pub struct ValueRangeStc<T: Arithmetic, LI: IntervalStc = Closed, UI: IntervalStc = Closed> {
    lb: Option<BoundStc<T, LI>>,
    ub: Option<BoundStc<T, UI>>,
}
