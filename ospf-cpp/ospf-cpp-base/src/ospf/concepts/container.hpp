#pragma once

#include <ospf/basic_definition.hpp>

namespace ospf
{
    inline namespace concepts
    {
        template<typename C>
        struct IsReservable
        {
            static constexpr const bool value = false;
        };

        template<typename C>
            requires requires (C& c) { c.reserve(std::declval<usize>()); }
        struct IsReservable<C>
        {
            static constexpr const bool value = true;
        };

        template<typename C>
        static constexpr const bool is_reservable = IsReservable<C>::value;

        template<typename C>
        concept Reservable = is_reservable<C>;
    };
};
