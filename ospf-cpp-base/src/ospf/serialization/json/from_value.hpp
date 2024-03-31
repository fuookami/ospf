#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/ospf_base_api.hpp>
#include <ospf/serialization/json/concepts.hpp>
#include <ospf/serialization/nullable.hpp>
#include <ospf/serialization/writable.hpp>
#include <deque>

namespace ospf
{
    inline namespace serialization
    {
        namespace json
        {
            OSPF_BASE_API std::optional<bool> to_bool(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<u8> to_u8(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<i8> to_i8(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<u16> to_u16(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<i16> to_i16(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<u32> to_u32(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<i32> to_i32(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<u64> to_u64(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<i64> to_i64(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<f32> to_f32(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<f64> to_f64(const rapidjson::Value& json) noexcept;
            OSPF_BASE_API std::optional<std::string> to_string(const rapidjson::Value& json) noexcept;

            // todo: big int, decimal and chrono

            template<typename T, CharType CharT>
            struct FromJsonValue;

            template<typename T, typename CharT>
            concept DeserializableFromJson = CharType<CharT> 
                && (requires (const FromJsonValue<OriginType<T>, CharT>& deserializer, T& obj)
                {
                    { deserializer(std::declval<Json<CharT>>(), obj, std::declval<std::optional<NameTransfer<CharT>>>()) } -> DecaySameAs<Try<>>;
                }) 
                && (!WithDefault<OriginType<T>> || requires (const FromJsonValue<OriginType<T>, CharT>& deserializer)
                {
                    { deserializer(std::declval<Json<CharT>>(), std::declval<std::optional<NameTransfer<CharT>>>()) } -> DecaySameAs<Result<OriginType<T>>>;
                });

            template<EnumType T, CharType CharT>
            struct FromJsonValue<T, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, T& value, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    OSPF_TRY_SET(value, this->operator()(json));
                    return succeed;
                }

                inline Result<T> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (json.IsString())
                    {
                        const auto enum_value = parse_enum<T, CharT>(json.GetString());
                        if (enum_value.has_value())
                        {
                            return *enum_value;
                        }
                    }
                    else if (json.IsUint64())
                    {
                        const auto enum_value = magic_enum::enum_cast<T>(json.GetUint64());
                        if (enum_value.has_value())
                        {
                            return *enum_value;
                        }
                    }
                    return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid value \"{}\" for enum \"{}\"", json, TypeInfo<T>::name()) };
                }
            };

            template<WithMetaInfo T, CharType CharT>
            struct FromJsonValue<T, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, T& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (json.IsNull())
                    {
                        if constexpr (serialization_nullable<T>)
                        {
                            return succeed;
                        }
                        else
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invlid json for type {}", json, TypeInfo<T>::name()) };
                        }
                    }
                    if (!json.IsObject())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invlid json for type {}", json, TypeInfo<T>::name()) };
                    }

                    static constexpr const meta_info::MetaInfo<T> info{};
                    std::optional<OSPFError> err;
                    info.for_each(obj, [&err, &json, &transfer](auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                            {
                                return;
                            }
                            else
                            {
                                static_assert(DeserializableFromJson<FieldValueType, CharT>);

                                if (err.has_value())
                                {
                                    return;
                                }

                                const auto key = transfer.has_value() ? (*transfer)(field.key()) : field.key();
                                if (!json.HasMember(key.data()))
                                {
                                    if constexpr (serialization_nullable<FieldValueType>)
                                    {
                                        return;
                                    }
                                    else
                                    {
                                        err = OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost non-nullable field \"{}\" for type {}", field.key(), TypeInfo<T>::name()) };
                                    }
                                }
                                else
                                {
                                    static const FromJsonValue<FieldValueType, CharT> deserializer{};
                                    auto value = deserializer(json[key.data()], transfer);
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
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return succeed;
                    }
                }

                template<typename = void>
                    requires WithDefault<T>
                inline Result<T> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    T obj = DefaultValue<T>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }
            };

            template<typename T, typename P, CharType CharT>
                requires WithoutMetaInfo<T> && DeserializableFromJson<T, CharT>
            struct FromJsonValue<NamedType<T, P>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, NamedType<T, P>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                    return deserializer(json, obj.unwrap(), transfer);
                }

                inline Result<NamedType<T, P>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    NamedType<T, P> obj = DefaultValue<NamedType<T, P>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromJson<T, CharT> && WithDefault<T>
            struct FromJsonValue<std::optional<T>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::optional<T>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (json.IsNull())
                    {
                        obj = std::nullopt;
                    }
                    else
                    {
                        static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_SET(obj, deserializer(json, transfer));
                    }
                    return succeed;
                }

                inline Result<std::optional<T>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::optional<T> obj{};
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }
            };

            template<typename T, PointerCategory cat, CharType CharT>
                requires DeserializableFromJson<T, CharT> && std::constructible_from<pointer::Ptr<T, cat>, PtrType<T>>
            struct FromJsonValue<pointer::Ptr<T, cat>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, pointer::Ptr<T, cat>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (json.IsNull())
                    {
                        obj = nullptr;
                    }
                    else
                    {
                        static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_GET(value, deserializer(json, transfer));
                        obj = pointer::Ptr<T, cat>{ new T{ std::move(value) } };
                    }
                    return succeed;
                }

                template<typename = void>
                    requires WithDefault<pointer::Ptr<T, cat>>
                inline Result<pointer::Ptr<T, cat>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    pointer::Ptr<T, cat> obj = DefaultValue<pointer::Ptr<T, cat>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }
            };

            template<typename T, typename U, CharType CharT>
                requires DeserializableFromJson<T, CharT>
            struct FromJsonValue<std::pair<T, U>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::pair<T, U>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::pair<T, U>>::name()) };
                    }
                    else
                    {
                        return deserialize<0_uz>(json.GetArray(), obj, transfer);
                    }
                }

                template<typename = void>
                    requires AllWithDefault<T, U>
                inline Result<std::pair<T, U>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::pair<T, U> obj = DefaultValue<std::pair<T, U>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }

            private:
                template<usize i>
                inline static Try<> deserialize(const ArrayView<CharT>& json, std::pair<T, U>& obj, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    if constexpr (i == 2_uz)
                    {
                        return succeed;
                    }
                    else if constexpr (i == 0_uz)
                    {
                        static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                        OSPF_TRY_EXEC(deserializer(json[i], obj.first, transfer));
                        return deserialize<i + 1_uz>(json, obj, transfer);
                    }
                    else if constexpr (i == 1_uz)
                    {
                        static const FromJsonValue<OriginType<U>, CharT> deserializer{};
                        OSPF_TRY_EXEC(deserializer(json[i], obj.second, transfer));
                        return deserialize<i + 1_uz>(json, obj, transfer);
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct FromJsonValue<std::tuple<Ts...>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::tuple<Ts...>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::tuple<Ts...>>::name()) };
                    }
                    else
                    {
                        return deserialize<0_uz>(json, obj, transfer);
                    }
                }

                template<typename = void>
                    requires AllWithDefault<Ts...>
                inline Result<std::tuple<Ts...>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::tuple<Ts...> obj = DefaultValue<std::tuple<Ts...>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }

            private:
                template<usize i>
                inline static Try<> deserialize(const ArrayView<CharT>& json, std::tuple<Ts...>& obj, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return succeed;
                    }
                    else
                    {
                        using ValueType = OriginType<decltype(std::get<i>(obj))>;
                        static_assert(DeserializableFromJson<ValueType, CharT>);
                        static const FromJsonValue<ValueType, CharT> deserializer{};
                        OSPF_TRY_EXEC(deserializer(json[i], std::get<i>(obj), transfer));
                        return deserialize<i + 1_uz>(json, obj, transfer);
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct FromJsonValue<std::variant<Ts...>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::variant<Ts...>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsObject())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        OSPF_TRY_GET(index, get_index(json, transfer));
                        static const auto key = transfer.has_value() ? (*transfer)("value") : boost::locale::conv::to_utf<CharT>("value", std::locale{});
                        if (!json.HasMember(key))
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost field \"value\" for \"{}\"", TypeInfo<std::variant<Ts...>>::name()) };
                        }
                        else
                        {
                            OSPF_TRY_EXEC(deserialize<0_uz>(json[key], obj, index, transfer));
                            return succeed;
                        }
                    }
                }

                template<typename = void>
                    requires AnyWithDefault<Ts...>
                inline Result<std::variant<Ts...>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::variant<Ts...> obj = DefaultValue<std::variant<Ts...>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }

            private:
                inline static Result<usize> get_index(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    static const auto key = transfer.has_value() ? (*transfer)("index") : boost::locale::conv::to_utf<CharT>("index", std::locale{});
                    if (!json.HasMember(key))
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost field \"index\" for \"{}\"", TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        const auto value = static_cast<usize>(to_u64(json[key]));
                        if (value.has_value())
                        {
                            return value;
                        }
                        else
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::variant<Ts...>>::name()) };
                        }
                    }
                }

                template<usize i>
                inline static Try<> deserialize(const Json<CharT>& json, std::variant<Ts...>& obj, const usize index, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::variant<Ts...>>::name()) };
                    }
                    else
                    {
                        if (i == index)
                        {
                            using ValueType = OriginType<decltype(std::get<i>(obj))>;
                            static_assert(DeserializableFromJson<ValueType, CharT>);
                            static const FromJsonValue<ValueType, CharT> deserializer{};
                            OSPF_TRY_GET(value, deserializer(json[i], transfer));
                            obj = std::variant<Ts...>{ std::in_place_index<i>, std::move(value) };
                            return succeed;
                        }
                        else
                        {
                            return deserialize<i + 1_uz>(json, obj, index, transfer);
                        }
                    }
                }
            };

            template<typename T, typename U, CharType CharT>
                requires DeserializableFromJson<T, CharT> && DeserializableFromJson<U, CharT>
            struct FromJsonValue<Either<T, U>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, Either<T, U>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsObject())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<Either<T, U>>::name()) };
                    }
                    else
                    {
                        OSPF_TRY_GET(index, get_index(json, transfer));
                        static const auto key = transfer.has_value() ? (*transfer)("value") : boost::locale::conv::to_utf<CharT>("index", std::locale{});
                        if (!json.HasMember(key))
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost field \"value\" for \"{}\"", TypeInfo<Either<T, U>>::name()) };
                        }
                        else
                        {
                            if (index == 0_uz)
                            {
                                static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                                OSPF_TRY_SET(obj, deserializer(json[key], transfer));
                            }
                            else
                            {
                                static const FromJsonValue<U, CharT> deserializer{};
                                OSPF_TRY_SET(obj, deserializer(json[key], transfer));
                            }
                            return succeed;
                        }
                    }
                }

                template<typename = void>
                    requires AnyWithDefault<T, U>
                inline Result<Either<T, U>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Either<T, U> obj = DefaultValue<Either<T, U>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }

            private:
                inline static Result<usize> get_index(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    static const auto key = transfer.has_value() ? (*transfer)("index") : boost::locale::conv::to_utf<CharT>("index", std::locale{});
                    if (!json.HasMember(key))
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost field \"index\" for \"{}\"", TypeInfo<Either<T, U>>::name()) };
                    }
                    else
                    {
                        const auto value = static_cast<usize>(to_u64(json[key]));
                        if (value.has_value())
                        {
                            return value;
                        }
                        else
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<Either<T, U>>::name()) };
                        }
                    }
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow, CharType CharT>
                requires DeserializableFromJson<T, CharT>
            struct FromJsonValue<ValOrRef<T, cat, cow>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, ValOrRef<T, cat, cow>& obj, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const FromJsonValue<OriginType<T>> deserializer{};
                    OSPF_TRY_GET(value, deserializer(json, transfer));
                    obj = ValOrRef<T, cat, cow>::value(std::move(value));
                    return succeed;
                }

                template<typename = void>
                    requires WithDefault<T>
                inline Result<ValOrRef<T, cat, cow>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    ValOrRef<T, cat, cow> obj = DefaultValue<ValOrRef<T, cat, cow>>::value();
                    OSPF_TRY_EXEC(this->operator()(json, obj, transfer));
                    return std::move(obj);
                }
            };

            template<typename T, usize len, CharType CharT>
                requires DeserializableFromJson<T, CharT> && WithDefault<T>
            struct FromJsonValue<std::array<T, len>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::array<T, len>& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::array<T, len>>::name()) };
                    }
                    else
                    {
                        usize i{ 0_uz };
                        for (const Json<CharT>& sub_json : json.GetArray())
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs[i] = std::move(obj);
                            ++i;

                            if (i == len)
                            {
                                break;
                            }
                        }
                        return succeed;
                    }
                }
                
                inline Result<std::array<T, len>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::array<T, len>>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if (json_array.Size() != len)
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid array length \"{}\" for \"{}\"", json_array.Size(), TypeInfo<std::array<T, len>>::name()) };
                        }
                        else
                        {
                            return make_array(boost::make_transform_iterator(json_array.Begin(), [&transfer](const Json<CharT>& sub_json) -> Result<T>
                                {
                                    static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                                    return deserializer(sub_json, transfer);
                                }));
                        }
                    }
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromJson<T, CharT> && WithDefault<T>
            struct FromJsonValue<std::vector<T>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::vector<T>& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::vector<T>>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        objs.reserve(objs.size() + json_array.Size());
                        for (const Json<CharT>& sub_json : json_array)
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.push_back(std::move(obj));
                        }
                        return succeed;
                    }
                }

                inline Result<std::vector<T>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::vector<T> objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
                    return std::move(objs);
                }
            };

            template<typename T, CharType CharT>
                requires DeserializableFromJson<T, CharT> && WithDefault<T>
            struct FromJsonValue<std::deque<T>, CharT>
            {
                inline Try<> operator()(const Json<CharT>& json, std::deque<T>& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<std::deque<T>>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        for (const Json<CharT>& sub_json : json_array)
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.push_back(std::move(obj));
                        }
                        return succeed;
                    }
                }

                inline Result<std::deque<T>> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    std::deque<T> objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
                    return std::move(objs);
                }
            };

            template<
                typename T,
                usize len,
                template<typename, usize> class C,
                CharType CharT
            >
                requires DeserializableFromJson<std::optional<T>, CharT>
            struct FromJsonValue<optional_array::StaticOptionalArray<T, len, C>, CharT>
            {
                using ArrayType = optional_array::StaticOptionalArray<T, len, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        usize i{ 0_uz };
                        for (const Json<CharT>& sub_json : json_array)
                        {
                            static const FromJsonValue<std::optional<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs[i] = std::move(obj);
                            ++i;

                            if (i == len)
                            {
                                break;
                            }
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if (json_array.Size() != len)
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid array length \"{}\" for \"{}\"", json_array.Size(), TypeInfo<ArrayType>::name()) };
                        }
                        OSPF_TRY_GET(objs, (make_array<std::optional<T>, len>(boost::make_transform_iterator(json_array.Begin(), [&transfer](const Json<CharT>& sub_json) -> Result<std::optional<T>>
                            {
                                static const FromJsonValue<std::optional<T>, CharT> deserializer{};
                                return deserializer(sub_json, transfer);
                            }))));
                        return ArrayType{ std::move(objs) };
                    }
                }
            };

            template<
                typename T,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromJson<std::optional<T>, CharT>
            struct FromJsonValue<optional_array::DynamicOptionalArray<T, C>, CharT>
            {
                using ArrayType = optional_array::DynamicOptionalArray<T, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if constexpr (Reservable<ArrayType>)
                        {
                            objs.reserve(json_array.Size());
                        } 
                        for (const auto& sub_json : json_array)
                        {
                            static const FromJsonValue<std::optional<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.push_back(std::move(obj));
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    ArrayType objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
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
                requires DeserializableFromJson<T, CharT> && WithDefault<pointer::Ptr<T, cat>>
            struct FromJsonValue<pointer_array::StaticPointerArray<T, len, cat, C>, CharT>
            {
                using ArrayType = pointer_array::StaticPointerArray<T, len, cat, C>;
                
                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        usize i{ 0_uz };
                        for (const Json<CharT>& sub_json : json_array)
                        {
                            static const FromJsonValue<pointer::Ptr<T, cat>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs[i] = std::move(obj);
                            ++i;

                            if (i == len)
                            {
                                break;
                            }
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if (json_array.Size() != len)
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid array length \"{}\" for \"{}\"", json_array.Size(), TypeInfo<ArrayType>::name()) };
                        }
                        OSPF_TRY_GET(objs, (make_array<pointer::Ptr<T, cat>, len>(boost::make_transform_iterator(json_array.Begin(), [&transfer](const Json<CharT>& sub_json) -> Result<pointer::Ptr<T, cat>>
                            {
                                static const FromJsonValue<pointer::Ptr<T, cat>, CharT> deserializer{};
                                return deserializer(sub_json, transfer);
                            }))));
                        return ArrayType{ std::move(objs) };
                    }
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromJson<T, CharT> && WithDefault<pointer::Ptr<T, cat>>
            struct FromJsonValue<pointer_array::DynamicPointerArray<T, cat, C>, CharT>
            {
                using ArrayType = pointer_array::DynamicPointerArray<T, cat, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if constexpr (Reservable<ArrayType>)
                        {
                            objs.reserve(json_array.Size());
                        }
                        for (const auto& sub_json : json_array)
                        {
                            static const FromJsonValue<pointer::Ptr<T, cat>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.push_back(std::move(obj));
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    ArrayType objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
                    return std::move(objs);
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C,
                CharType CharT
            >
                requires DeserializableFromJson<T, CharT>
            struct FromJsonValue<tagged_map::TaggedMap<T, E, C>, CharT>
            {
                using ArrayType = tagged_map::TaggedMap<T, E, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        for (const auto& sub_json : json_array)
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.insert(std::move(obj));
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    ArrayType objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
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
                requires DeserializableFromJson<T, CharT>
            struct FromJsonValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>, CharT>
            {
                using ArrayType = value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        usize i{ 0_uz };
                        for (const Json<CharT>& sub_json : json_array)
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs[i] = ValOrRef<T, cat, cow>::value(std::move(obj));
                            ++i;

                            if (i == len)
                            {
                                break;
                            }
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if (json_array.Size() != len)
                        {
                            return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid array length \"{}\" for \"{}\"", json_array.Size(), TypeInfo<ArrayType>::name()) };
                        }
                        OSPF_TRY_GET(objs, (make_array<ValOrRef<T, cat, cow>, len>(boost::make_transform_iterator(json_array.Begin(), [&transfer](const Json<CharT>& sub_json) -> Result<ValOrRef<T, cat, cow>>
                            {
                                static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                                OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                                return ValOrRef<T, cat, cow>::value(std::move(obj));
                            }))));
                        return ArrayType{ std::move(objs) };
                    }
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C,
                CharType CharT
            >
                requires DeserializableFromJson<T, CharT>
            struct FromJsonValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>, CharT>
            {
                using ArrayType = value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>;

                inline Try<> operator()(const Json<CharT>& json, ArrayType& objs, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    if (!json.IsArray())
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for \"{}\"", json, TypeInfo<ArrayType>::name()) };
                    }
                    else
                    {
                        const ArrayView<CharT>& json_array = json.GetArray();
                        if constexpr (Reservable<ArrayType>)
                        {
                            objs.reserve(json_array.Size());
                        }
                        for (const auto& sub_json : json_array)
                        {
                            static const FromJsonValue<OriginType<T>, CharT> deserializer{};
                            OSPF_TRY_GET(obj, deserializer(sub_json, transfer));
                            objs.push_back(ValOrRef<T, cat>::value(std::move(obj)));
                        }
                        return succeed;
                    }
                }

                inline Result<ArrayType> operator()(const Json<CharT>& json, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    ArrayType objs;
                    OSPF_TRY_EXEC(this->operator()(json, objs, transfer));
                    return std::move(objs);
                }
            };

            template<>
            struct FromJsonValue<bool, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, bool& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto bool_value = to_bool(json);
                    if (bool_value.has_value())
                    {
                        value = bool_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for bool", json) };
                    }
                }

                inline Result<bool> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    bool value{ true };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<u8, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, u8& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto u8_value = to_u8(json);
                    if (u8_value.has_value())
                    {
                        value = u8_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for u8", json) };
                    }
                }

                inline Result<u8> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    u8 value{ 0_u8 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<i8, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, i8& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto i8_value = to_i8(json);
                    if (i8_value.has_value())
                    {
                        value = i8_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for i8", json) };
                    }
                }

                inline Result<i8> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    i8 value{ 0_i8 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<u16, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, u16& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto u16_value = to_u16(json);
                    if (u16_value.has_value())
                    {
                        value = u16_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for u16", json) };
                    }
                }

                inline Result<u16> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    u16 value{ 0_u16 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<i16, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, i16& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto i16_value = to_i16(json);
                    if (i16_value.has_value())
                    {
                        value = i16_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for i16", json) };
                    }
                }

                inline Result<i16> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    i16 value{ 0_i16 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<u32, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, u32& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto u32_value = to_u32(json);
                    if (u32_value.has_value())
                    {
                        value = u32_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for u32", json) };
                    }
                }

                inline Result<u32> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    u32 value{ 0_u32 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<i32, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, i32& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto i32_value = to_i32(json);
                    if (i32_value.has_value())
                    {
                        value = i32_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for i32", json) };
                    }
                }

                inline Result<i32> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    i32 value{ 0_i32 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<u64, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, u64& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto u64_value = to_u64(json);
                    if (u64_value.has_value())
                    {
                        value = u64_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for u64", json) };
                    }
                }

                inline Result<u64> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    u64 value{ 0_u64 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<i64, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, i64& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto i64_value = to_i64(json);
                    if (i64_value.has_value())
                    {
                        value = i64_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for i64", json) };
                    }
                }

                inline Result<i64> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    i64 value{ 0_i64 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<f32, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, f32& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto f32_value = to_f32(json);
                    if (f32_value.has_value())
                    {
                        value = f32_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for f32", json) };
                    }
                }

                inline Result<f32> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    f32 value{ 0._f32 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<f64, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, f64& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto f64_value = to_f64(json);
                    if (f64_value.has_value())
                    {
                        value = f64_value.value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for f64", json) };
                    }
                }

                inline Result<f64> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    f64 value{ 0._f64 };
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };

            template<>
            struct FromJsonValue<std::string, char>
            {
                inline Try<> operator()(const rapidjson::Value& json, std::string& value, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    const auto u64_value = to_string(json);
                    if (u64_value.has_value())
                    {
                        value = std::move(u64_value).value();
                        return succeed;
                    }
                    else
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid json \"{}\" for std::string", json) };
                    }
                }

                inline Result<std::string> operator()(const rapidjson::Value& json, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    std::string value{};
                    OSPF_TRY_EXEC(this->operator()(json, value, transfer));
                    return value;
                }
            };
        };
    };
};
