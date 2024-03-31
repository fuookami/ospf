#pragma once

#include <ospf/type_family.hpp>
#include <ospf/concepts/base.hpp>

namespace ospf
{
    inline namespace concepts
    {
        template<typename T>
        struct TagValue;

        template<typename T>
        concept WithTag = std::default_initializable<TagValue<T>> 
            && requires(const TagValue<T>& trait, const T& ele)
            {
                { trait.value(ele) } -> DecaySameAs<typename TagValue<T>::Type>;
            };
    };
};
