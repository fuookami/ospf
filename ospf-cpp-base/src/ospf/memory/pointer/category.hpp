#pragma once

#include <ospf/basic_definition.hpp>

namespace ospf
{
    inline namespace memory
    {
        enum class PointerCategory : u8
        {
            Unique,
            Shared,
            Weak,
            Raw
        };

        namespace pointer
        {
            template<typename T, PointerCategory cat = PointerCategory::Raw>
            class Ptr;
        };
    };
};

namespace std
{
    template<typename T, ospf::PointerCategory cat>
        requires requires(ospf::pointer::Ptr<T, cat>& lhs, ospf::pointer::Ptr<T, cat>& rhs) { lhs.swap(rhs); }
    inline void swap(ospf::pointer::Ptr<T, cat>& lhs, ospf::pointer::Ptr<T, cat>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};
