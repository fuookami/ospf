#include <ospf/math/algebra/operator/comparison/zero.hpp>

namespace ospf::math::algebra::comparison_operator
{
    template class Zero<i8>;
    template class Zero<u8>;
    template class Zero<i16>;
    template class Zero<u16>;
    template class Zero<i32>;
    template class Zero<u32>;
    template class Zero<i64>;
    template class Zero<u64>;
    template class Zero<i128>;
    template class Zero<u128>;
    template class Zero<i256>;
    template class Zero<u256>;
    template class Zero<i512>;
    template class Zero<u512>;
    template class Zero<i1024>;
    template class Zero<u1024>;
    template class Zero<bigint>;

    template class Zero<f32>;
    template class Zero<f64>;
    template class Zero<f128>;
    template class Zero<f256>;
    template class Zero<f512>;
    template class Zero<dec50>;
    template class Zero<dec100>;
};
