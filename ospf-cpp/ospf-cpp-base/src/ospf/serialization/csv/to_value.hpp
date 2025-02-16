#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/reference_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/meta_programming/named_type.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/ospf_base_api.hpp>
#include <ospf/serialization/csv/concepts.hpp>
#include <deque>
#include <span>

namespace ospf
{
    inline namespace serialization
    {
        namespace csv
        {
            OSPF_BASE_API std::string from_bool(const bool value) noexcept;
            OSPF_BASE_API std::string from_u8(const u8 value) noexcept;
            OSPF_BASE_API std::string from_i8(const i8 value) noexcept;
            OSPF_BASE_API std::string from_u16(const u16 value) noexcept;
            OSPF_BASE_API std::string from_i16(const i16 value) noexcept;
            OSPF_BASE_API std::string from_u32(const u32 value) noexcept;
            OSPF_BASE_API std::string from_i32(const i32 value) noexcept;
            OSPF_BASE_API std::string from_u64(const u64 value) noexcept;
            OSPF_BASE_API std::string from_i64(const i64 value) noexcept;
            OSPF_BASE_API std::string from_f32(const f32 value) noexcept;
            OSPF_BASE_API std::string from_f64(const f64 value) noexcept;

            // todo: big int, decimal and chrono
            // todo: impl for different character

            template<typename T, CharType CharT>
            struct ToCSVValue;

            template<typename T, typename CharT>
            concept SerializableToCSV = CharType<CharT> 
                && requires (std::basic_ostringstream<CharT>& os, const ToCSVValue<OriginType<T>, CharT>& serializer)
                {
                    { serializer(std::declval<OriginType<T>>()) } -> DecaySameAs<Result<std::basic_string<CharT>>>;
                    { serializer(os, std::declval<OriginType<T>>()) } -> DecaySameAs<Try<>>;
                };

            template<EnumType T, CharType CharT>
            struct ToCSVValue<T, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const T value) const noexcept
                {
                    return to_string<T, CharT>(value);
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const T value) const noexcept
                {
                    os << to_string<T, CharT>(value);
                    return succeed;
                }
            };

            template<typename T, CharType CharT>
                requires (std::convertible_to<T, std::basic_string<CharT>> || std::convertible_to<T, std::basic_string_view<CharT>>)
            struct ToCSVValue<T, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(ArgCLRefType<T> value) const noexcept
                {
                    if constexpr (std::convertible_to<T, std::basic_string_view<CharT>>)
                    {
                        return std::basic_string<CharT>{ static_cast<std::basic_string_view<CharT>>(value) };
                    }
                    else
                    {
                        return static_cast<std::basic_string<CharT>>(value);
                    }
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, ArgCLRefType<T> value) const noexcept
                {
                    if constexpr (std::convertible_to<T, std::basic_string_view<CharT>>)
                    {
                        os << static_cast<std::basic_string_view<CharT>>(value);
                    }
                    else
                    {
                        os << static_cast<std::basic_string<CharT>>(value);
                    }
                    return succeed;
                }
            };

            template<typename T, typename P, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<NamedType<T, P>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(ArgCLRefType<NamedType<T, P>> value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(value.unwrap());
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, ArgCLRefType<NamedType<T, P>> value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(value.unwrap());
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<std::optional<T>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::optional<T>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    if (value.has_value())
                    {
                        return serializer(*value);
                    }
                    else
                    {
                        return std::basic_string<CharT>{};
                    }
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::optional<T>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    if (value.has_value())
                    {
                        return serializer(os, *value);
                    }
                    else
                    {
                        return succeed;
                    }
                }
            };

            template<typename T, PointerCategory cat, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<pointer::Ptr<T, cat>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const pointer::Ptr<T, cat>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    if (value != nullptr)
                    {
                        return serializer(*value);
                    }
                    else
                    {
                        return std::basic_string<CharT>{};
                    }
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const pointer::Ptr<T, cat>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    if (value != nullptr)
                    {
                        return serializer(os, *value);
                    }
                    else
                    {
                        return succeed;
                    }
                }
            };

            template<typename T, ReferenceCategory cat, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<reference::Ref<T, cat>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const reference::Ref<T, cat>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(*value);
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const reference::Ref<T, cat>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(os, *value);
                }
            };

            template<typename T, typename U, CharType CharT>
                requires SerializableToCSV<T, CharT> && SerializableToCSV<U, CharT>
            struct ToCSVValue<std::pair<T, U>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::pair<T, U>& value) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    static const ToCSVValue<OriginType<T>, CharT> serializer1{};
                    OSPF_TRY_GET(value1, serializer1(value.first));
                    sout << std::move(value1);
                    sout << CharTrait<CharT>::default_sub_seperator;
                    static const ToCSVValue<OriginType<U>, CharT> serializer2{};
                    OSPF_TRY_GET(value2, serializer2(value.second));
                    sout << std::move(value2);
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::pair<T, U>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer1{};
                    OSPF_TRY_GET(value1, serializer1(value.first));
                    os << std::move(value1);
                    os << CharTrait<CharT>::default_sub_seperator;
                    static const ToCSVValue<OriginType<U>, CharT> serializer2{};
                    OSPF_TRY_GET(value2, serializer2(value.second));
                    os << std::move(value2);
                    return succeed;
                }
            };

            template<typename... Ts, CharType CharT>
            struct ToCSVValue<std::tuple<Ts...>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::tuple<Ts...>& value) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(serialize<0_uz>(sout, value));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::tuple<Ts...>& value) noexcept
                {
                    return serialize<0_uz>(os, value);
                }

            private:
                template<usize i>
                inline static Try<> serialize(std::basic_ostringstream<CharT>& os, const std::tuple<Ts...>& value) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return succeed;
                    }
                    else
                    {
                        using ValueType = OriginType<decltype(std::get<i>(value))>;
                        static_assert(SerializableToCSV<ValueType, CharT>);
                        static const ToCSVValue<ValueType, CharT> serializer{};
                        std::basic_ostringstream<CharT> sout;
                        OSPF_TRY_GET(this_value, serializer(sout, std::get<i>(value)));
                        sout << std::move(this_value);
                        if constexpr (i != (VariableTypeList<Ts...>::length - 1_uz))
                        {
                            sout << CharTrait<CharT>::default_sub_seperator;
                        }
                        return serialize<i + 1_uz>(os, value);
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct ToCSVValue<std::variant<Ts...>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::variant<Ts...>& value) const noexcept
                {
                    std::basic_ostringstream sout;
                    OSPF_TRY_EXEC(this->operator()(sout, value));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::variant<Ts...>& value) const noexcept
                {
                    os << value.index();
                    os << CharTrait<CharT>::default_sub_seperator;
                    OSPF_TRY_EXEC(std::visit([&os](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToCSV<ValueType, CharT>);
                            static const ToCSVValue<ValueType, CharT> serializer{};
                            return serializer(os, this_value);
                        }, value));
                    return succeed;
                }
            };

            template<typename T, typename U, CharType CharT>
                requires SerializableToCSV<T, CharT> && SerializableToCSV<U, CharT>
            struct ToCSVValue<Either<T, U>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const Either<T, U>& value) const noexcept
                {
                    std::basic_ostringstream sout;
                    OSPF_TRY_EXEC(this->operator()(sout, value));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const Either<T, U>& value) const noexcept
                {
                    os << value.is_left() ? 0_uz : 1_uz;
                    os << CharTrait<CharT>::default_sub_seperator;
                    OSPF_TRY_EXEC(std::visit([&os](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToCSV<ValueType, CharT>);
                            static const ToCSVValue<ValueType, CharT> serializer{};
                            return serializer(os, this_value);
                        }, value));
                    return succeed;
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<ValOrRef<T, cat, cow>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const ValOrRef<T, cat, cow>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(*value);
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const ValOrRef<T, cat, cow>& value) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    return serializer(os, *value);
                }
            };

            template<typename T, usize len, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<std::array<T, len>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::array<T, len>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::array<T, len>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<std::vector<T>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::vector<T>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::vector<T>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<std::deque<T>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::deque<T>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::deque<T>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<typename T, usize len, CharType CharT>
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<std::span<const T, len>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const std::span<const T, len> values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const std::span<const T, len> values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToCSV<std::optional<T>, CharT>
            struct ToCSVValue<optional_array::StaticOptionalArray<T, len, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const optional_array::StaticOptionalArray<T, len, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const optional_array::StaticOptionalArray<T, len, C>& values) const noexcept
                {
                    static const ToCSVValue<std::optional<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToCSV<std::optional<T>, CharT>
            struct ToCSVValue<optional_array::DynamicOptionalArray<T, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const optional_array::DynamicOptionalArray<T, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const optional_array::DynamicOptionalArray<T, C>& values) const noexcept
                {
                    static const ToCSVValue<std::optional<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToCSV<pointer::Ptr<T, cat>, CharT>
            struct ToCSVValue<pointer_array::StaticPointerArray<T, len, cat, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const pointer_array::StaticPointerArray<T, len, cat, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const pointer_array::StaticPointerArray<T, len, cat, C>& values) const noexcept
                {
                    static const ToCSVValue<pointer::Ptr<T, cat>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToCSV<pointer::Ptr<T, cat>, CharT>
            struct ToCSVValue<pointer_array::DynamicPointerArray<T, cat, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const pointer_array::DynamicPointerArray<T, cat, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const pointer_array::DynamicPointerArray<T, cat, C>& values) const noexcept
                {
                    static const ToCSVValue<pointer::Ptr<T, cat>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<reference_array::StaticReferenceArray<T, len, cat, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const reference_array::StaticReferenceArray<T, len, cat, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const reference_array::StaticReferenceArray<T, len, cat, C>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, *values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<reference_array::DynamicReferenceArray<T, cat, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const reference_array::DynamicReferenceArray<T, cat, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const reference_array::DynamicReferenceArray<T, cat, C>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, *values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C,
                CharType CharT
            >
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<tagged_map::TaggedMap<T, E, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const tagged_map::TaggedMap<T, E, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const tagged_map::TaggedMap<T, E, C>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    usize i{ 0_uz };
                    for (const auto& value : values)
                    {
                        OSPF_TRY_EXEC(serializer(os, *value));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                        ++i;
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, *values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToCSV<T, CharT>
            struct ToCSVValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>, CharT>
            {
                inline Result<std::basic_string<CharT>> operator()(const value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>& values) const noexcept
                {
                    std::basic_ostringstream<CharT> sout;
                    OSPF_TRY_EXEC(this->operator()(sout, values));
                    return sout.str();
                }

                inline Try<> operator()(std::basic_ostringstream<CharT>& os, const value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>& values) const noexcept
                {
                    static const ToCSVValue<OriginType<T>, CharT> serializer{};
                    for (usize i{ 0_uz }; i != values.size(); ++i)
                    {
                        OSPF_TRY_EXEC(serializer(os, *values[i]));
                        if (i != (values.size() - 1_uz))
                        {
                            os << CharTrait<CharT>::default_seperator;
                        }
                    }
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<bool, char>
            {
                inline Result<std::string> operator()(const bool value) const noexcept
                {
                    return from_bool(value);
                }

                inline Try<> operator()(std::ostringstream& os, const bool value) const noexcept
                {
                    os << (value ? "true" : "false");
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<u8, char>
            {
                inline Result<std::string> operator()(const u8 value) const noexcept
                {
                    return from_u8(value);
                }

                inline Try<> operator()(std::ostringstream& os, const u8 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<i8, char>
            {
                inline Result<std::string> operator()(const i8 value) const noexcept
                {
                    return from_i8(value);
                }

                inline Try<> operator()(std::ostringstream& os, const i8 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<u16, char>
            {
                inline Result<std::string> operator()(const u16 value) const noexcept
                {
                    return from_u16(value);
                }

                inline Try<> operator()(std::ostringstream& os, const u16 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<i16, char>
            {
                inline Result<std::string> operator()(const i16 value) const noexcept
                {
                    return from_i16(value);
                }

                inline Try<> operator()(std::ostringstream& os, const i16 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<u32, char>
            {
                inline Result<std::string> operator()(const u32 value) const noexcept
                {
                    return from_u32(value);
                }

                inline Try<> operator()(std::ostringstream& os, const u32 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<i32, char>
            {
                inline Result<std::string> operator()(const i32 value) const noexcept
                {
                    return from_i32(value);
                }

                inline Try<> operator()(std::ostringstream& os, const i32 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<u64, char>
            {
                inline Result<std::string> operator()(const u64 value) const noexcept
                {
                    return from_u64(value);
                }

                inline Try<> operator()(std::ostringstream& os, const u64 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<i64, char>
            {
                inline Result<std::string> operator()(const i64 value) const noexcept
                {
                    return from_i64(value);
                }

                inline Try<> operator()(std::ostringstream& os, const i16 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<f32, char>
            {
                inline Result<std::string> operator()(const f32 value) const noexcept
                {
                    return from_f32(value);
                }

                inline Try<> operator()(std::ostringstream& os, const f32 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<f64, char>
            {
                inline Result<std::string> operator()(const f64 value) const noexcept
                {
                    return from_f64(value);
                }

                inline Try<> operator()(std::ostringstream& os, const f64 value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<std::string, char>
            {
                inline Result<std::string> operator()(const std::string& value) const noexcept
                {
                    return value;
                }

                inline Try<> operator()(std::ostringstream& os, const std::string& value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };

            template<>
            struct ToCSVValue<std::string_view, char>
            {
                inline Result<std::string> operator()(const std::string_view value) const noexcept
                {
                    return std::string{ value };
                }

                inline Try<> operator()(std::ostringstream& os, const std::string_view value) const noexcept
                {
                    os << value;
                    return succeed;
                }
            };
        };
    };
};
