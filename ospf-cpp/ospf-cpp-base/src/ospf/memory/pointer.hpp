#pragma once

#include <ospf/memory/pointer/raw.hpp>
#include <ospf/memory/pointer/unique.hpp>
#include <ospf/memory/pointer/shared.hpp>
#include <ospf/memory/pointer/weak.hpp>

namespace ospf
{
    inline namespace memory
    {
        template<
            typename T,
            PointerCategory cat = PointerCategory::Raw
        >
        using Ptr = std::conditional_t<
            std::is_array_v<T>,
            pointer::Ptr<ArrayType<T>, cat>,
            pointer::Ptr<OriginType<T>, cat>
        >;

        template<typename T>
        using Unique = std::conditional_t<
            std::is_array_v<T>,
            pointer::Ptr<ArrayType<T>, PointerCategory::Unique>,
            pointer::Ptr<OriginType<T>, PointerCategory::Unique>
        >;

        template<typename T>
        using Shared = std::conditional_t<
            std::is_array_v<T>,
            pointer::Ptr<ArrayType<T>, PointerCategory::Shared>,
            pointer::Ptr<OriginType<T>, PointerCategory::Shared>
        >;

        template<typename T>
        using Weak = std::conditional_t<
            std::is_array_v<T>,
            pointer::Ptr<ArrayType<T>, PointerCategory::Weak>,
            pointer::Ptr<OriginType<T>, PointerCategory::Weak>
        >;
    };
};
