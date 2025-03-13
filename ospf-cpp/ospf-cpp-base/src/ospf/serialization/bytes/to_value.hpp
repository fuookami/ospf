#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/reference_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/bytes/bytes.hpp>
#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/serialization/bytes/concepts.hpp>
#include <deque>
#include <span>

namespace ospf
{
    inline namespace serialization
    {
        namespace bytes
        {
            template<typename T>
            struct ToBytesValue;

            template<typename T>
            concept SerializableToBytes = requires (const ToBytesValue<OriginType<T>> serializer, Bytes<>::iterator it)
            {
                { serializer.size(std::declval<OriginType<T>>()) } -> DecaySameAs<usize>;
                { serializer(std::declval<OriginType<T>>(), it, std::declval<Endian>()) } -> DecaySameAs<Try<>>;
            };

            template<EnumType T>
                requires SerializableToBytes<magic_enum::underlying_type_t<T>>
            struct ToBytesValue<T>
            {
                using ValueType = magic_enum::underlying_type_t<T>;

                inline const usize size(const T value) const noexcept
                {
                    return sizeof(ValueType) / address_length;
                }

                template<ToValueIter It>
                inline Try<> operator()(const T value, It& it, const Endian endian) const noexcept
                {
                    static const ToBytesValue<ValueType> serializer{};
                    return serializer(static_cast<ValueType>(value, it, endian));
                }
            };

            template<WithMetaInfo T>
            struct ToBytesValue<T>
            {
                inline const usize size(const T& value) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<T> info{};
                    usize ret{ 0_uz };
                    info.for_each(value, [&ret](const auto& obj, const auto& field) 
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            static_assert(SerializableToBytes<FieldValueType>);
                            static const ToBytesValue<FieldValueType> serializer{};
                            ret += serializer.size(field.value(obj));
                        });
                    return ret;
                }

                template<ToValueIter It>
                inline Try<> operator()(const T& value, It& it, const Endian endian) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<T> info{};
                    info.for_each(value, [&it](const auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            static_assert(SerializableToBytes<FieldValueType>);
                            static const ToBytesValue<FieldValueType> serializer{};
                            serializer(field.value(obj), it, endian);
                        });
                    return succeed;
                }
            };

            template<typename T, typename P>
                requires SerializableToBytes<T>
            struct ToBytesValue<NamedType<T, P>>
            {
                inline const usize size(const NamedType<T, P>& value) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer.size(value.unwrap());
                }

                template<ToValueIter It>
                inline Try<> operator()(const NamedType<T, P>& value, It& it, const Endian endian) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer(value.unwrap(), it, endian);
                }
            };

            template<typename T>
                requires SerializableToBytes<T>
            struct ToBytesValue<std::optional<T>>
            {
                inline const usize size(const std::optional<T>& value) const noexcept
                {
                    if (value.has_value())
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        return 1_uz + serializer.size(*value);
                    }
                    else
                    {
                        return 1_uz;
                    }
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::optional<T>& value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<bool>(value.has_value(), it, endian);
                    if (value.has_value())
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(*value, it, endian));
                    }
                    return succeed;
                }
            };

            template<typename T, PointerCategory cat>
                requires SerializableToBytes<T>
            struct ToBytesValue<pointer::Ptr<T, cat>>
            {
                inline const usize size(const pointer::Ptr<T, cat>& value) const noexcept
                {
                    if (value != nullptr)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        return 1_uz + serializer.size(*value);
                    }
                    else
                    {
                        return 1_uz;
                    }
                }

                template<ToValueIter It>
                inline Try<> operator()(const pointer::Ptr<T, cat>& value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<bool>(value != nullptr, it, endian);
                    if (value != nullptr)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(*value, it, endian));
                    }
                    return succeed;
                }
            };
            
            template<typename T, ReferenceCategory cat>
                requires SerializableToBytes<T>
            struct ToBytesValue<reference::Ref<T, cat>>
            {
                inline const usize size(const reference::Ref<T, cat>& value) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer.size(*value);
                }

                template<ToValueIter It>
                inline Try<> operator()(const reference::Ref<T, cat>& value, It& it, const Endian endian) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer(*value, it, endian);
                }
            };

            template<typename T, typename U>
                requires SerializableToBytes<T> && SerializableToBytes<U>
            struct ToBytesValue<std::pair<T, U>>
            {
                inline const usize size(const std::pair<T, U>& value) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer1{};
                    static const ToBytesValue<OriginType<U>> serializer2{};
                    return serializer1.size(value.first) + serializer2.size(value.second);
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::pair<T, U>& value, It& it, const Endian endian) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer1{};
                    static const ToBytesValue<OriginType<U>> serializer2{};
                    OSPF_TRY_EXEC(serializer1(value.first, it, endian));
                    OSPF_TRY_EXEC(serializer2(value.second, it, endian));
                    return succeed;
                }
            };

            template<typename... Ts>
            struct ToBytesValue<std::tuple<Ts...>>
            {
                inline const usize size(const std::tuple<Ts...>& value) const noexcept
                {
                    return get_size<0_uz>(0_uz, value);
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::tuple<Ts...>& value, It& it, const Endian endian) const noexcept
                {
                    return serialize<0_uz>(value, it, endian);
                }

            private:
                template<usize i>
                inline static const usize get_size(const usize curr_size, const std::tuple<Ts...>& value) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return curr_size;
                    }
                    else
                    {
                        using ValueType = OriginType<decltype(std::get<i>(value))>;
                        static_assert(SerializableToBytes<ValueType>);
                        static const ToBytesValue<ValueType> serializer{};
                        return curr_size + serializer.size(std::get<i>(value));
                    }
                }

                template<usize i, ToValueIter It>
                inline static Try<> serialize(const std::tuple<Ts...>& value, It& it, const Endian endian) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return succeed;
                    }
                    else
                    {
                        using ValueType = OriginType<decltype(std::get<i>(value))>;
                        static_assert(SerializableToBytes<ValueType>);
                        static const ToBytesValue<ValueType> serializer{};
                        OSPF_TRY_EXEC(serializer(std::get<i>(value), it, endian));
                        return serialize<i + 1_uz>(value, it, endian);
                    }
                }
            };

            template<typename... Ts>
            struct ToBytesValue<std::variant<Ts...>>
            {
                inline const usize size(const std::variant<Ts...>& value) const noexcept
                {
                    return address_length + std::visit([](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToBytes<ValueType>);
                            static const ToBytesValue<ValueType> serializer{};
                            return serializer.size(this_value);
                        }, value);
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::variant<Ts...>& value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(value.index(), it, endian);
                    return std::visit([](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToBytes<ValueType>);
                            static const ToBytesValue<ValueType> serializer{};
                            return serializer(this_value, it, endian);
                        });
                }
            };

            template<typename T, typename U>
                requires SerializableToBytes<T> && SerializableToBytes<U>
            struct ToBytesValue<Either<T, U>>
            {
                inline const usize size(const Either<T, U>& value) const noexcept
                {
                    return 1_uz + std::visit([](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToBytes<ValueType>);
                            static const ToBytesValue<ValueType> serializer{};
                            return serializer.size(this_value);
                        }, value);
                }

                template<ToValueIter It>
                inline Try<> operator()(const Either<T, U>& value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<bool>(value.is_left(), it, endian);
                    return std::visit([](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToBytes<ValueType>);
                            static const ToBytesValue<ValueType> serializer{};
                        }, value);
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow>
                requires SerializableToBytes<T>
            struct ToBytesValue<ValOrRef<T, cat, cow>>
            {
                inline const usize size(const ValOrRef<T, cat, cow>& value) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer.size(*value);
                }

                template<ToValueIter It>
                inline Try<> operator()(const ValOrRef<T, cat, cow>& value, It& it, const Endian endian) const noexcept
                {
                    static const ToBytesValue<OriginType<T>> serializer{};
                    return serializer(*value, it, endian);
                }
            };

            template<typename T, usize len>
                requires SerializableToBytes<T>
            struct ToBytesValue<std::array<T, len>>
            {
                inline const usize size(const std::array<T, len>& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::array<T, len>& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<typename T>
                requires SerializableToBytes<T>
            struct ToBytesValue<std::vector<T>>
            {
                inline const usize size(const std::vector<T>& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::vector<T>& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<typename T>
                requires SerializableToBytes<T>
            struct ToBytesValue<std::deque<T>>
            {
                inline const usize size(const std::deque<T>& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::deque<T>& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<typename T, usize len>
                requires SerializableToBytes<T>
            struct ToBytesValue<std::span<const T, len>>
            {
                inline const usize size(const std::span<const T, len> values) const noexcept
                {
                    return address_length + std::accumulate(values.begin(), values.end(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::span<const T, len> values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T, 
                usize len,
                template<typename, usize> class C
            >
                requires SerializableToBytes<std::optional<T>>
            struct ToBytesValue<optional_array::StaticOptionalArray<T, len, C>>
            {
                using ArrayType = optional_array::StaticOptionalArray<T, len, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<std::optional<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<std::optional<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                template<typename> class C
            >
                requires SerializableToBytes<std::optional<T>>
            struct ToBytesValue<optional_array::DynamicOptionalArray<T, C>>
            {
                using ArrayType = optional_array::DynamicOptionalArray<T, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<std::optional<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<std::optional<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C
            >
                requires SerializableToBytes<pointer::Ptr<T, cat>>
            struct ToBytesValue<pointer_array::StaticPointerArray<T, len, cat, C>>
            {
                using ArrayType = pointer_array::StaticPointerArray<T, len, cat, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<pointer::Ptr<T, cat>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<pointer::Ptr<T, cat>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C
            >
                requires SerializableToBytes<pointer::Ptr<T, cat>>
            struct ToBytesValue<pointer_array::DynamicPointerArray<T, cat, C>>
            {
                using ArrayType = pointer_array::DynamicPointerArray<T, cat, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<pointer::Ptr<T, cat>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<pointer::Ptr<T, cat>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                template<typename, usize> class C
            >
                requires SerializableToBytes<T>
            struct ToBytesValue<reference_array::StaticReferenceArray<T, len, cat, C>>
            {
                using ArrayType = reference_array::StaticReferenceArray<T, len, cat, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                template<typename> class C
            >
                requires SerializableToBytes<T>
            struct ToBytesValue<reference_array::DynamicReferenceArray<T, cat, C>>
            {
                using ArrayType = reference_array::DynamicReferenceArray<T, cat, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires SerializableToBytes<T>
            struct ToBytesValue<tagged_map::TaggedMap<T, E, C>>
            {
                using ArrayType = tagged_map::TaggedMap<T, E, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename, usize> class C
            >
                requires SerializableToBytes<T>
            struct ToBytesValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>>
            {
                using ArrayType = value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs) 
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C
            >
                requires SerializableToBytes<T>
            struct ToBytesValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>>
            {
                using ArrayType = value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>;

                inline const usize size(const ArrayType& values) const noexcept
                {
                    return address_length + std::accumulate(values.cbegin(), values.cend(), 0_uz, [](const usize lhs, const auto& rhs)
                        {
                            static const ToBytesValue<OriginType<T>> serializer{};
                            return lhs + serializer.size(rhs);
                        });
                }

                template<ToValueIter It>
                inline Try<> operator()(const ArrayType& values, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(values.size(), it, endian);
                    for (const auto& value : values)
                    {
                        static const ToBytesValue<OriginType<T>> serializer{};
                        OSPF_TRY_EXEC(serializer(value, it, endian));
                    }
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<bool>
            {
                inline const usize size(const bool value) const noexcept
                {
                    return 1_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const bool value, It& it, const Endian endian) const noexcept
                {
                    *it = value ? 1_ub : 0_ub;
                    ++it;
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<u8>
            {
                inline const usize size(const u8 value) const noexcept
                {
                    return 1_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const u8 value, It& it, const Endian endian) const noexcept
                {
                    *it = static_cast<ubyte>(value);
                    ++it;
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<i8>
            {
                inline const usize size(const i8 value) const noexcept
                {
                    return 1_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const i8 value, It& it, const Endian endian) const noexcept
                {
                    *it = static_cast<ubyte>(value);
                    ++it;
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<u16>
            {
                inline const usize size(const u16 value) const noexcept
                {
                    return 2_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const u16 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<u16>(value, it, endian);
                    return succeed;
                };
            };

            template<>
            struct ToBytesValue<i16>
            {
                inline const usize size(const i16 value) const noexcept
                {
                    return 2_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const i16 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<i16>(value, it, endian);
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<u32>
            {
                inline const usize size(const u32 value) const noexcept
                {
                    return 4_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const u32 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<u32>(value, it, endian);
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<i32>
            {
                inline const usize size(const i32 value) const noexcept
                {
                    return 4_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const i32 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<i32>(value, it, endian);
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<u64>
            {
                inline const usize size(const u64 value) const noexcept
                {
                    return 8_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const u64 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<u64>(value, it, endian);
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<i64>
            {
                inline const usize size(const i64 value) const noexcept
                {
                    return 8_uz;
                }

                template<ToValueIter It>
                inline Try<> operator()(const i64 value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<i64>(value, it, endian);
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<std::string>
            {
                inline const usize size(const std::string value) const noexcept
                {
                    return address_length + value.size();
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::string& value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(value.size(), it, endian);
                    std::transform(value.cbegin(), value.cend(), it, [](const char ch) { return static_cast<ubyte>(ch); });
                    return succeed;
                }
            };

            template<>
            struct ToBytesValue<std::string_view>
            {
                inline const usize size(const std::string_view value) const noexcept
                {
                    return address_length + value.size();
                }

                template<ToValueIter It>
                inline Try<> operator()(const std::string_view value, It& it, const Endian endian) const noexcept
                {
                    to_bytes<usize>(value.size(), it, endian);
                    std::transform(value.cbegin(), value.cend(), it, [](const char ch) { return static_cast<ubyte>(ch); });
                    return succeed;
                }
            };

            // todo: array<bool>, vector<bool>, deque<bool>, span<bool>, big int, decimal and chrono
        };
    };
};
