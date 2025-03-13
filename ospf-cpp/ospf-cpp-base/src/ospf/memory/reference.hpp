#pragma once

#include <ospf/memory/reference/raw.hpp>
#include <ospf/memory/reference/borrow.hpp>
#include <ospf/memory/reference/unique_borrow.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace reference
        {
            template<typename T, ReferenceCategory cat>
            struct ReferenceTrait
            {
                using Type = reference::Ref<OriginType<T>, cat>;
            };

            template<typename T, ReferenceCategory cat>
            struct ReferenceTrait<reference::Ref<T, cat>, cat>
                : public ReferenceTrait<T, cat> {};
        };

        template<
            typename T,
            ReferenceCategory cat = ReferenceCategory::Reference
        >
        using Ref = typename reference::ReferenceTrait<OriginType<T>, cat>::Type;

        template<typename T>
        using Borrow = typename reference::Ref<OriginType<T>, ReferenceCategory::Borrow>::Type;

        template<typename T>
        using UniqueBorrow = typename reference::Ref<OriginType<T>, ReferenceCategory::UniqueBorrow>::Type;
    };
};

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2>
inline ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<ospf::reference::Ref<T, cat1>>& lhs, const std::optional<ospf::reference::Ref<U, cat2>>& rhs) noexcept
{
    if (static_cast<bool>(lhs) && static_cast<bool>(rhs))
    {
        return **lhs <=> **rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}

template<typename T, typename U, ospf::ReferenceCategory cat1, ospf::ReferenceCategory cat2>
inline ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<const ospf::reference::Ref<T, cat1>>& lhs, const std::optional<const ospf::reference::Ref<U, cat2>>& rhs) noexcept
{
    if (static_cast<bool>(lhs) && static_cast<bool>(rhs))
    {
        return **lhs <=> **rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> static_cast<bool>(rhs);
    }
}
