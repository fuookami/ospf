#pragma once

#include <ospf/memory/pointer.hpp>
#include <optional>
#include <vector>
#include <deque>

namespace ospf
{
    inline namespace serialization
    {
        template<typename T>
        struct SerializationNullableTrait
        {
            static constexpr const bool value = false;
        };

        template<typename T>
        static constexpr const bool serialization_nullable = SerializationNullableTrait<T>::value;

        template<typename T>
        concept SerializationNullable = serialization_nullable<T>;

        template<typename T>
        struct SerializationNullableTrait<std::optional<T>>
        {
            static constexpr const bool value = true;
        };

        template<typename T, PointerCategory cat>
            requires WithDefault<pointer::Ptr<T, cat>>
        struct SerializationNullableTrait<pointer::Ptr<T, cat>>
        {
            static constexpr const bool value = true;
        };

        template<typename T>
        struct SerializationNullableTrait<std::vector<T>>
        {
            static constexpr const bool value = true;
        };

        template<typename T>
        struct SerializationNullableTrait<std::deque<T>>
        {
            static constexpr const bool value = true;
        };
    };
};
