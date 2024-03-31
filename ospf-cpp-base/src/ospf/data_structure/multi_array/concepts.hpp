#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/data_structure/multi_array/shape.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        static constexpr const auto dynamic_dimension = npos;

        namespace multi_array
        {
            template<
                typename T,
                usize dim
            >
                requires NotSameAs<T, void>
            class MultiArray;

            // for Einstein Notation
            template<
                typename A,
                typename S
            >
                requires NotSameAs<typename A::ValueType, void>
            class MultiArrayView;

            template<
                typename T,
                usize dim
            >
                requires NotSameAs<T, void> && (dim != dynamic_dimension)
            class MultiArrayMapView;
        };
    };
};
