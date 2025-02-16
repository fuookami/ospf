#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/meta_programming/named_type.hpp>
#include <ospf/meta_programming/type_info.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/ospf_base_api.hpp>
#include <ospf/serialization/csv/concepts.hpp>
#include <ospf/serialization/nullable.hpp>
#include <ospf/serialization/writable.hpp>
#include <deque>

namespace ospf
{
    inline namespace serialization
    {
        namespace csv
        {
            OSPF_BASE_API std::optional<bool> to_bool(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<u8> to_u8(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<i8> to_i8(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<u16> to_u16(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<i16> to_i16(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<i32> to_i32(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<u32> to_u32(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<i64> to_i64(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<u64> to_u64(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<f32> to_f32(const std::string_view value) noexcept;
            OSPF_BASE_API std::optional<f64> to_f64(const std::string_view value) noexcept;

            // todo: big int, decimal and chrono
            // todo: impl for different character

            template<typename T, CharType CharT>
            struct FromCSVValue;

            template<typename T, typename CharT>
            concept DeserializableFromCSV = CharType<CharT>
                && requires (const FromCSVValue<OriginType<T>, CharT>& deserializer)
                {
                    { deserializer(std::declval<std::basic_string_view<CharT>>()) } -> DecaySameAs<Result<OriginType<T>>>;
                };

            template<EnumType T, CharType CharT>
            struct FromCSVValue<T, CharT>
            {
                inline Result<T> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto enum_value = parse_enum<T, CharT>(value);
                    if (enum_value.has_value())
                    {
                        return *enum_value;
                    }
                    const auto int_value = to_u64(value);
                    if (int_value.has_value())
                    {
                        const auto enum_value = magic_enum::enum_cast<T>(int_value);
                        if (enum_value.has_value())
                        {
                            return *enum_value;
                        }
                    }
                    return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for enum \"{}\"", value, TypeInfo<T>::name()) };
                }
            };

            template<typename T, CharType CharT>
                requires (std::convertible_to<std::basic_string<CharT>, T> || std::convertible_to<std::basic_string_view<CharT>, T>)
            struct FromCSVValue<T, CharT>
            {
                inline Result<T> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    if constexpr (std::convertible_to<std::basic_string_view<CharT>, T>)
                    {
                        return static_cast<T>(value);
                    }
                    else
                    {
                        return static_cast<T>(std::basic_string<CharT>{ value });
                    }
                }
            };

            template<typename T, typename P, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<NamedType<T, P>, CharT>
            {
                inline Result<NamedType<T, P>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                    OSPF_TRY_GET(obj, deserializer(value));
                    return NamedType<T, P>{ std::move(obj) };
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<std::optional<T>, CharT>
            {
                inline Result<std::optional<T>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                    const auto obj = deserializer(value);
                    if (obj.is_failed())
                    {
                        if (value.empty())
                        {
                            return std::optional<T>{};
                        }
                        else
                        {
                            return std::move(obj).err();
                        }
                    }
                    else
                    {
                        return std::optional<T>{ std::move(obj).unwrap() };
                    }
                }
            };

            template<typename T, PointerCategory cat, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<pointer::Ptr<T, cat>, CharT>
            {
                inline Result<pointer::Ptr<T, cat>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                    const auto obj = deserializer(value);
                    if (obj.is_failed())
                    {
                        if (value.empty())
                        {
                            return pointer::Ptr<T, cat>{};
                        }
                        else
                        {
                            return std::move(obj).err();
                        }
                    }
                    else
                    {
                        return pointer::Ptr<T, cat>{ new T{ std::move(obj).unwrap() } };
                    }
                }
            };

            template<typename T, typename U, CharType CharT>
                requires DeserializableFromCSV<T, CharT> && DeserializableFromCSV<U, CharT>
            struct FromCSVValue<std::pair<T, U>, CharT>
            {
                inline Result<std::pair<T, U>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_sub_seperator);
                    if (strs.size() != 2_uz)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::pair<T, U>>::name()) };
                    }
                    else
                    {
                        static const FromCSVValue<OriginType<T>, CharT> deserializer1{};
                        static const FromCSVValue<OriginType<U>, CharT> deserializer2{};
                        OSPF_TRY_GET(value1, deserializer1(strs[0_uz]));
                        OSPF_TRY_GET(value2, deserializer2(strs[1_uz]));
                        return std::make_pair(std::move(value1), std::move(value2));
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct FromCSVValue<std::tuple<Ts...>, CharT>
            {
                inline Result<std::tuple<Ts...>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_sub_seperator);
                    if (strs.size() != VariableTypeList<Ts...>::length)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::tuple<Ts...>>::name()) };
                    }
                    else
                    {
                        return deserialize<0_uz>(strs);
                    }
                }

            private:
                template<usize i, usize len, typename... Args>
                inline static Result<std::tuple<Ts...>> deserialize(const std::span<const std::basic_string_view<CharT>, len> strs, Args&&... args) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return std::tuple{ std::forward<Args>(args)... };
                    }
                    else
                    {
                        using ValueType = OriginType<TypeAt<i, Ts...>>;
                        static_assert(DeserializableFromCSV<ValueType, CharT>);
                        static const FromCSVValue<ValueType, CharT> deserializer{};
                        return deserialize<i + 1_uz>(strs, std::forward<Args>(args)..., deserializer(strs[i]));
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct FromCSVValue<std::variant<Ts...>, CharT>
            {
                inline Result<std::variant<Ts...>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_sub_seperator);
                    if (strs.size() != 2_uz)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        const auto index = to_u64(strs[0_uz]);
                        if (!index.has_value() || *index != VariableTypeList<Ts...>::length)
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::variant<Ts...>>::name()) };
                        }
                        else
                        {
                            return deserialize<0_uz>(index, strs[1_uz]);
                        }
                    }
                }

            private:
                template<usize i>
                inline static Result<std::variant<Ts...>> deserialize(const usize index, const std::basic_string_view<CharT> value) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        if (i == index)
                        {
                            using ValueType = OriginType<TypeAt<i, Ts...>>;
                            static_assert(DeserializableFromCSV<ValueType, CharT>);
                            static const FromCSVValue<ValueType, CharT> deserializer{};
                            OSPF_TRY_GET(obj, Deserializer(value));
                            return std::variant<Ts...>{ std::in_place_index<i>, std::move(obj) };
                        }
                        else
                        {
                            return deserialize<i + 1_uz>(index, value);
                        }
                    }
                }
            };

            template<typename T, typename U, CharType CharT>
                requires DeserializableFromCSV<T, CharT> && DeserializableFromCSV<U, CharT>
            struct FromCSVValue<Either<T, U>, CharT>
            {
                inline Result<Either<T, U>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_sub_seperator);
                    if (strs.size() != 2_uz)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<Either<T, U>>::name()) };
                    }
                    else
                    {
                        const auto index = to_u64(strs[0_uz]);
                        if (!index.has_value())
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<Either<T, U>>::name()) };
                        }
                        else
                        {
                            switch (*index)
                            {
                            case 0_uz:
                            {
                                static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                                OSPF_TRY_GET(obj, deserializer(strs[1_uz]));
                                return Either<T, U>::left(std::move(obj));
                            }
                            case 1_uz:
                            {
                                static const FromCSVValue<OriginType<U>, CharT> deserializer{};
                                OSPF_TRY_GET(obj, deserializer(strs[1_uz]));
                                return Either<T, U>::right(std::move(obj));
                            }
                            default:
                                break;
                            }
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<Either<T, U>>::name()) };
                        }
                    }
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<ValOrRef<T, cat, cow>, CharT>
            {
                inline Result<ValOrRef<T, cat, cow>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                    OSPF_TRY_GET(obj, deserializer(value));
                    return ValOrRef<T, cat, cow>::value(std::move(obj));
                }
            };

            template<typename T, usize len, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<std::array<T, len>, CharT>
            {
                inline Result<std::array<T, len>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    if (strs.size() != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<std::array<T, len>>::name()) };
                    }
                    else
                    {
                        return make_array<T, len>([&strs](const usize i) -> Result<T>
                            {
                                static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                                return deserializer(strs[i]);
                            });
                    }
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<std::vector<T>, CharT>
            {
                inline Result<std::vector<T>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    std::vector<T> objs;
                    objs.reserve(strs.size());
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<std::deque<T>, CharT>
            {
                inline Result<std::deque<T>> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    std::deque<T> objs;
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<
                typename T,
                usize len,
                template<typename, usize> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<std::optional<T>, CharT>
            struct FromCSVValue<optional_array::StaticOptionalArray<T, len, C>, CharT>
            {
                using ArrayType = optional_array::StaticOptionalArray<T, len, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    if (strs.size() != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<std::optional<T>, len>([&strs](const usize i) -> Result<std::optional<T>>
                        {
                            static const FromCSVValue<std::optional<T>, CharT> deserializer{};
                            return deserializer(strs[i]);
                        })));
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<std::optional<T>, CharT>
            struct FromCSVValue<optional_array::DynamicOptionalArray<T, C>, CharT>
            {
                using ArrayType = optional_array::DynamicOptionalArray<T, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(strs.size());
                    }
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<std::optional<T>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<pointer::Ptr<T, cat>, CharT>
            struct FromCSVValue<pointer_array::StaticPointerArray<T, len, cat, C>, CharT>
            {
                using ArrayType = pointer_array::StaticPointerArray<T, len, cat, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    if (strs.size() != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<pointer::Ptr<T, cat>, len>([&strs](const usize i) -> Result<pointer::Ptr<T, cat>>
                        {
                            static const FromCSVValue<pointer::Ptr<T, cat>, CharT> deserializer{};
                            return deserializer(strs[i]);
                        })));
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<pointer::Ptr<T, cat>, CharT>
            struct FromCSVValue<pointer_array::DynamicPointerArray<T, cat, C>, CharT>
            {
                using ArrayType = pointer_array::DynamicPointerArray<T, cat, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(strs.size());
                    }
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<pointer::Ptr<T, cat>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<tagged_map::TaggedMap<T, E, C>, CharT>
            {
                using ArrayType = tagged_map::TaggedMap<T, E, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    ArrayType objs;
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.insert(std::move(obj));
                    }
                    return std::move(objs);
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
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>, CharT>
            {
                using ArrayType = value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    if (strs.size() != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for \"{}\"", value, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<ValOrRef<T, cat, cow>, len>([&strs](const usize i) -> Result<ValOrRef<T, cat, cow>>
                        {
                            static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(strs[i]));
                            return ValOrRef<T, cat, cow>::value(std::move(obj));
                        })));
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromCSV<T, CharT>
            struct FromCSVValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>, CharT>
            {
                using ArrayType = value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>;

                inline Result<ArrayType> operator()(const std::basic_string_view<CharT> value) const noexcept
                {
                    const auto strs = split(value, CharTrait<CharT>::default_seperator);
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(strs.size());
                    }
                    for (const auto str : strs)
                    {
                        static const FromCSVValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(str));
                        objs.push_back(ValOrRef<T, cat, cow>::value(std::move(obj)));
                    }
                    return std::move(objs);
                }
            };

            template<>
            struct FromCSVValue<bool, char>
            {
                inline Result<bool> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_bool(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for bool", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<u8, char>
            {
                inline Result<u8> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_u8(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for u8", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<i8, char>
            {
                inline Result<i8> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_i8(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for i8", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<u16, char>
            {
                inline Result<u16> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_u16(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for u16", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<i16, char>
            {
                inline Result<i16> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_i16(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for i16", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<u32, char>
            {
                inline Result<u32> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_u32(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for u32", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<i32, char>
            {
                inline Result<i32> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_i32(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for i32", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<u64, char>
            {
                inline Result<u64> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_u64(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for u64", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<i64, char>
            {
                inline Result<i64> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_i64(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for i64", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<f32, char>
            {
                inline Result<f32> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_f32(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for f32", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<f64, char>
            {
                inline Result<f64> operator()(const std::string_view value) const noexcept
                {
                    const auto ret = to_f64(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for f64", value) };
                    }
                }
            };

            template<>
            struct FromCSVValue<std::string, char>
            {
                inline Result<std::string> operator()(const std::string_view value) const noexcept
                {
                    return std::string{ value };
                }
            };

            template<>
            struct FromCSVValue<std::string_view, char>
            {
                inline Result<std::string_view> operator()(const std::string_view value) const noexcept
                {
                    return value;
                }
            };
        };
    };
};
