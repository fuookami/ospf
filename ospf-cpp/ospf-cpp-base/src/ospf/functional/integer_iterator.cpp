#include "integer_iterator.hpp"

namespace ospf::functional
{
    template class IntegerIterator<i32>;
    template class IntegerIterator<u32>;
    template class IntegerIterator<usize>;
    template class IntegerIterator<isize>;
};
