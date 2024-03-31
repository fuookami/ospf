#include <ospf/functional/range_bounds.hpp>

namespace ospf::function
{
    template class Bound<i32>;
    template class Bound<u32>;
    template class Bound<usize>;
    template class Bound<isize>;
    template class RangeBounds<i32>;
    template class RangeBounds<u32>;
    template class RangeBounds<usize>;
    template class RangeBounds<isize>;
};
