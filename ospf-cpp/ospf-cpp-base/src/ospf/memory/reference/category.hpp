#pragma once

#include <ospf/basic_definition.hpp>

namespace ospf
{
    inline namespace memory
    {
        enum class ReferenceCategory : u8
        {
            Reference,      // not null pointer
            Borrow,
            UniqueBorrow
        };

        namespace reference
        {
            template<typename T, ReferenceCategory cat = ReferenceCategory::Reference>
            class Ref;
        };
    };
};

namespace std
{
    template<typename T, ospf::ReferenceCategory cat>
        requires requires(ospf::reference::Ref<T, cat>& lhs, ospf::reference::Ref<T, cat>& rhs) { lhs.swap(rhs); }
    inline void swap(ospf::reference::Ref<T, cat>& lhs, ospf::reference::Ref<T, cat>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};

