#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/functional/range_bounds.hpp>
#include <ospf/bytes/bytes.hpp>
#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/serialization/bytes/concepts.hpp>
#include <ospf/serialization/writable.hpp>
#include <ospf/serialization/nullable.hpp>
#include <deque>

namespace ospf
{
    inline namespace serialization
    {
        namespace bytes
        {
            template<typename T>
            struct FromBytesValue;

            template<typename T>
            concept DeserializableFromBytes = requires (const FromBytesValue<OriginType<T>> deserializer, Bytes<>::iterator it, const usize address_length, const Endian endian)
            {
                { deserializer(it, address_length, endian) } -> DecaySameAs<Result<OriginType<T>>>;
            };

            template<FromValueIter It>
            inline Result<usize> get_size(It& it, const usize address_length, const Endian endian) noexcept
            {
                switch (address_length)
                {
                case 1_uz:
                    return static_cast<usize>(from_bytes<u8>(it, endian));
                case 2_uz:
                    return static_cast<usize>(from_bytes<u16>(it, endian));
                case 4_uz:
                    return static_cast<usize>(from_bytes<u32>(it, endian));
                case 8_uz:
                    return static_cast<usize>(from_bytes<u64>(it, endian));
                default:
                    break;
                }
                return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid address length: \"{}\"", address_length) };
            }

            template<EnumType T>
                requires DeserializableFromBytes<magic_enum::underlying_type_t<T>>
            struct FromBytesValue<T>
            {
                using ValueType = magic_enum::underlying_type_t<T>;

                template<FromValueIter It>
                inline Result<T> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    static const FromBytesValue<ValueType> deserializer{};
                    OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                    const auto ret = magic_enum::enum_cast(value);
                    if (ret.has_value())
                    {
                        return *ret;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for enum \"{}\"", value, TypeInfo<T>::name()) };
                    }
                }
            };

            template<WithMetaInfo T>
                requires WithDefault<T>
            struct FromBytesValue<T>
            {
                template<FromValueIter It>
                inline Result<T> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    static const meta_info::MetaInfo<OriginType<T>> info{};
                    T obj = DefaultValue<T>::value();
                    std::optional<OSPFError> err;
                    info.for_each(obj, [&err, &it, address_length, endian](auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                            {
                                return;
                            }
                            else
                            {
                                static_assert(DeserializableFromBytes<FieldValueType>);

                                if (err.has_value())
                                {
                                    return;
                                }

                                static const FromBytesValue<FieldValueType> deserializer{};
                                auto value = deserializer(it, address_length, endian);
                                if constexpr (!serialization_nullable<FieldValueType>)
                                {
                                    if (value.is_failed())
                                    {
                                        err = OSPFError{ OSPFErrCode::DeserializationFail, std::format("failed deserializing field \"{}\" for type {}, {}", field.key(), TypeInfo<T>::name(), value.err().message()) };
                                        return;
                                    }
                                }
                                if (value.is_succeeded())
                                {
                                    field.value(obj) = std::move(value).unwrap();
                                }
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return std::move(obj);
                    }
                }
            };

            template<typename T, typename P>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<NamedType<T, P>>
            {
                template<FromValueIter It>
                inline Result<NamedType<T, P>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    static const FromBytesValue<OriginType<T>> deserializer{};
                    OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                    return NamedType<T, P>{ std::move(value) };
                }
            };

            template<typename T>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<std::optional<T>>
            {
                template<FromValueIter It>
                inline Result<std::optional<T>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    const bool has_value = from_bytes<bool>(it, endian);
                    if (has_value)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                        return std::optional<T>{ std::move(value) };
                    }
                    else
                    {
                        return std::optional<T>{ std::nullopt };
                    }
                }
            };

            template<typename T, PointerCategory cat>
                requires DeserializableFromBytes<T> && WithDefault<pointer::Ptr<T, cat>>
            struct FromBytesValue<pointer::Ptr<T, cat>>
            {
                template<FromValueIter It>
                inline Result<pointer::Ptr<T, cat>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    const bool not_null = from_bytes<bool>(it, endian);
                    if (not_null)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                        return pointer::Ptr<T, cat>{ new T{ std::move(value) } };
                    }
                    else
                    {
                        return pointer::Ptr<T, cat>{};
                    }
                }
            };

            template<typename T, typename U>
                requires DeserializableFromBytes<T> && DeserializableFromBytes<U>
            struct FromBytesValue<std::pair<T, U>>
            {
                template<FromValueIter It>
                inline Result<std::pair<T, U>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    static const FromBytesValue<OriginType<T>> deserializer1{};
                    static const FromBytesValue<U> deserializer1{};
                    OSPF_TRY_GET(value1, deserializer1(it, address_length, endian));
                    OSPF_TRY_GET(value2, deserializer2(it, address_length, endian));
                    return std::pair<T, U>{ std::move(value1), std::move(value2) };
                }
            };

            template<typename... Ts>
            struct FromBytesValue<std::tuple<Ts...>>
            {
                template<FromValueIter It>
                inline Result<std::tuple<Ts...>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return deserialize<0_uz>(it, address_length, endian);
                }

            private:
                template<usize i, FromValueIter It, typename... Args>
                inline static Result<std::tuple<Ts...>> deserialize(It& it, const usize address_length, const Endian endian, Args&&... args) noexcept
                {
                    if (i == VariableTypeList<Ts...>::length)
                    {
                        return std::tuple<Ts...>{ std::forward<Args>(args)... };
                    }
                    else
                    {
                        using ValueType = OriginType<TypeAt<i, Ts...>>;
                        static_assert(DeserializableFromBytes<ValueType>);
                        static const FromBytesValue<ValueType> deserializer;
                        OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                        return deserialize<i + 1_uz>(it, endian, std::forward<Args>(args)..., std::move(value));
                    }
                }
            };

            template<typename... Ts>
            struct FromBytesValue<std::variant<Ts...>>
            {
                template<FromValueIter It>
                inline Result<std::variant<Ts...>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(index, get_size(it, address_length, endian));
                    return deserialize<0_uz>(index, it, address_length, endian);
                }

            private:
                template<usize i, FromValueIter It>
                inline static Result<std::variant<Ts...>> deserialize(const usize index, It& it, const usize address_length, const Endian endian) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid index \"{}\" for \"{}\"", index, TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        if (i == index)
                        {
                            using ValueType = OriginType<TypeAt<i, Ts...>>;
                            static_assert(DeserializableFromBytes<ValueType>);
                            static const FromBytesValue<ValueType> deserializer;
                            OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                            return std::variant<Ts...>{ std::in_place_index<i>, std::move(value) };
                        }
                        else
                        {
                            return deserialize<i + 1_uz>(index, it, endian);
                        }
                    }
                }
            };

            template<typename T, typename U>
                requires DeserializableFromBytes<T> && DeserializableFromBytes<U>
            struct FromBytesValue<Either<T, U>>
            {
                template<FromValueIter It>
                inline Result<Either<T, U>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    const auto is_left = from_bytes<bool>(it, endian);
                    if (is_left)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                        return Either<T, U>::left(std::move(value));
                    }
                    else
                    {
                        static const FromBytesValue<OriginType<U>> deserializer{};
                        OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                        return Either<T, U>::right(std::move(value));
                    }
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<ValOrRef<T, cat, cow>>
            {
                template<FromValueIter It>
                inline Result<ValOrRef<T, cat, cow>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    static const FromBytesValue<OriginType<T>> deserializer{};
                    OSPF_TRY_GET(value, deserializer(it, address_length, endian));
                    return ValOrRef<T, cat, cow>::value(std::move(value));
                }
            };

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<std::array<T, len>>
            {
                template<FromValueIter It>
                inline Result<std::array<T, len>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    if (size != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid size \"{}\" for \"{}\"", size, TypeInfo<std::array<T, len>>::name()) };
                    }
                    return make_array<T, len>([&it, address_length, endian](const usize _) -> Result<T>
                        {
                            static const FromBytesValue<OriginType<T>> deserializer{};
                            return deserializer(it, address_length, endian);
                        });
                }
            };

            template<typename T>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<std::vector<T>>
            {
                template<FromValueIter It>
                inline Result<std::vector<T>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    std::vector<T> objs;
                    objs.reserve(size);
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<typename T>
                requires DeserializableFromBytes<T>
            struct FromBytesValue<std::deque<T>>
            {
                template<FromValueIter It>
                inline Result<std::deque<T>> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    std::deque<T> objs;
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                        objs.push_back(std::move(obj));
                    }
                    return std::move(objs);
                }
            };

            template<
                typename T, 
                usize len,
                template<typename, usize> class C
            >
                requires DeserializableFromBytes<std::optional<T>>
            struct FromBytesValue<optional_array::StaticOptionalArray<T, len, C>>
            {
                using ArrayType = optional_array::StaticOptionalArray<T, len, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    if (size != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid size \"{}\" for \"{}\"", size, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<std::optional<T>, len>([&it, address_length, endian](const usize _) -> Result<std::optional<T>>
                        {
                            static const FromBytesValue<std::optional<T>> deserializer{};
                            return deserializer(it, address_length, endian);
                        })));
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                template<typename> class C
            >
                requires DeserializableFromBytes<std::optional<T>>
            struct FromBytesValue<optional_array::DynamicOptionalArray<T, C>>
            {
                using ArrayType = optional_array::DynamicOptionalArray<T, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(size);
                    }
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<std::optional<T>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                        objs.push_back(std::move(obj));
                    }
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C
            >
                requires DeserializableFromBytes<pointer::Ptr<T, cat>>
            struct FromBytesValue<pointer_array::StaticPointerArray<T, len, cat, C>>
            {
                using ArrayType = pointer_array::StaticPointerArray<T, len, cat, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    if (size != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid size \"{}\" for \"{}\"", size, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<pointer::Ptr<T, cat>, len>([&it, address_length, endian](const usize _) -> Result<pointer::Ptr<T, cat>>
                        {
                            static const FromBytesValue<pointer::Ptr<T, cat>> deserializer{};
                            return deserializer(it, address_length, endian);
                        })));
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C
            >
                requires DeserializableFromBytes<pointer::Ptr<T, cat>>
            struct FromBytesValue<pointer_array::DynamicPointerArray<T, cat, C>>
            {
                using ArrayType = pointer_array::DynamicPointerArray<T, cat, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(size);
                    }
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<pointer::Ptr<T, cat>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                        objs.push_back(std::move(obj));
                    }
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires DeserializableFromBytes<T>
            struct FromBytesValue<tagged_map::TaggedMap<T, E, C>>
            {
                using ArrayType = tagged_map::TaggedMap<T, E, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    ArrayType objs;
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
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
                template<typename, usize> class C
            >
                requires DeserializableFromBytes<T>
            struct FromBytesValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>>
            {
                using ArrayType = value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    if (size != len)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid size \"{}\" for \"{}\"", size, TypeInfo<ArrayType>::name()) };
                    }
                    OSPF_TRY_GET(objs, (make_array<ValOrRef<T, cat, cow>, len>([&it, address_length, endian](const usize _) -> Result<ValOrRef<T, cat, cow>>
                        {
                            static const FromBytesValue<OriginType<T>> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                            return ValOrRef<T, cat, cow>::value(std::move(obj));
                        })));
                    return ArrayType{ std::move(objs) };
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C
            >
                requires DeserializableFromBytes<T>
            struct FromBytesValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>>
            {
                using ArrayType = value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>;

                template<FromValueIter It>
                inline Result<ArrayType> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    ArrayType objs;
                    if constexpr (Reservable<ArrayType>)
                    {
                        objs.reserve(size);
                    }
                    for (const auto _ : 0_uz RTo size)
                    {
                        static const FromBytesValue<OriginType<T>> deserializer{};
                        OSPF_TRY_GET(obj, deserializer(it, address_length, endian));
                        objs.push_back(ValOrRef<T, cat, cow>::value(std::move(obj)));
                    }
                    return std::move(objs);
                }
            };

            template<>
            struct FromBytesValue<bool>
            {
                template<FromValueIter It>
                inline Result<bool> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<bool>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<u8>
            {
                template<FromValueIter It>
                inline Result<u8> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<u8>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<i8>
            {
                template<FromValueIter It>
                inline Result<i8> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<i8>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<u16>
            {
                template<FromValueIter It>
                inline Result<u16> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<u16>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<i16>
            {
                template<FromValueIter It>
                inline Result<i16> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<i16>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<u32>
            {
                template<FromValueIter It>
                inline Result<u32> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<u32>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<i32>
            {
                template<FromValueIter It>
                inline Result<i32> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<i32>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<u64>
            {
                template<FromValueIter It>
                inline Result<u64> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<u64>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<i64>
            {
                template<FromValueIter It>
                inline Result<i64> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    return from_bytes<i64>(it, endian);
                }
            };

            template<>
            struct FromBytesValue<std::string>
            {
                template<FromValueIter It>
                inline Result<std::string> operator()(It& it, const usize address_length, const Endian endian) const noexcept
                {
                    OSPF_TRY_GET(size, get_size(it, address_length, endian));
                    std::string str{ size, '\0' };
                    for (usize i{ 0_uz }; i != size; ++i)
                    {
                        str[i] = static_cast<char>(from_bytes<ubyte>(it, endian));
                    }
                    return std::move(str);
                }
            };
        };
    };
};
