#pragma once

#include <ospf/data_structure/optional_array.hpp>
#include <ospf/data_structure/pointer_array.hpp>
#include <ospf/data_structure/reference_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/data_structure/value_or_reference_array.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/ospf_base_api.hpp>
#include <ospf/serialization/json/concepts.hpp>
#include <deque>
#include <span>

namespace ospf
{
    inline namespace serialization
    {
        namespace json
        {
            OSPF_BASE_API rapidjson::Value from_bool(const bool value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_u8(const u8 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_i8(const i8 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_u16(const u16 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_i16(const i16 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_u32(const u32 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_i32(const i32 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_u64(const u64 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_i64(const i64 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_f32(const f32 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_f64(const f64 value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_string(const std::string& value, rapidjson::Document& doc) noexcept;
            OSPF_BASE_API rapidjson::Value from_string_view(const std::string_view value, rapidjson::Document& doc) noexcept;

            // todo: big int, decimal and chrono

            template<typename T, CharType CharT>
            struct ToJsonValue;

            template<typename T, typename CharT>
            concept SerializableToJson = CharType<CharT> 
                && std::default_initializable<ToJsonValue<OriginType<T>, CharT>>
                && requires (const ToJsonValue<OriginType<T>, CharT> serializer, Document<CharT>& doc)
                {
                    { serializer(std::declval<OriginType<T>>(), doc, std::declval<std::optional<NameTransfer<CharT>>>()) } -> DecaySameAs<Result<Json<CharT>>>;
                };

            template<EnumType T, CharType CharT>
            struct ToJsonValue<T, CharT>
            {
                inline Result<Json<CharT>> operator()(const T value, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    return Json<CharT>{ to_string<T, CharT>(value).data(), doc.GetAllocator() };
                }
            };

            template<WithMetaInfo T, CharType CharT>
            struct ToJsonValue<T, CharT>
            {
                inline Result<Json<CharT>> operator()(const T& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<T> info{};
                    Json<CharT> json{ rapidjson::kObjectType };
                    std::optional<OSPFError> err;
                    info.for_each(obj, [&json, &err, &doc, &transfer](const auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            static_assert(SerializableToJson<FieldValueType, CharT>);

                            if (err.has_value())
                            {
                                return;
                            }

                            const auto key = transfer.has_value() ? (*transfer)(field.key()) : field.key();
                            static const ToJsonValue<FieldValueType, CharT> serializer{};
                            auto sub_json = serializer(field.value(obj), doc, transfer);
                            if (sub_json.is_failed())
                            {
                                err = OSPFError{ OSPFErrCode::SerializationFail, std::format("failed serializing field \"{}\" for type\"{}\", {}", field.key(), TypeInfo<T>::name(), sub_json.err().message()) };
                                return;
                            }
                            else
                            {
                                json.AddMember(rapidjson::StringRef(key.data()), sub_json.unwrap().Move(), doc.GetAllocator());
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return std::move(json);
                    }
                }
            };

            template<typename T, typename P, CharType CharT>
                requires WithoutMetaInfo<T> && SerializableToJson<T, CharT>
            struct ToJsonValue<NamedType<T, P>, CharT>
            {
                inline Result<Json<CharT>> operator()(const NamedType<T, P>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    return serializer(obj.unwrap(), doc, transfer);
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<std::optional<T>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::optional<T>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    if (obj.has_value())
                    {
                        return serializer(*obj, doc, transfer);
                    }
                    else
                    {
                        return Json<CharT>{ rapidjson::kNullType };
                    }
                }
            };

            template<typename T, PointerCategory cat, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<pointer::Ptr<T, cat>, CharT>
            {
                inline Result<Json<CharT>> operator()(const pointer::Ptr<T, cat>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    if (obj != nullptr)
                    {
                        return serializer(*obj, doc, transfer);
                    }
                    else
                    {
                        return Json<CharT>{ rapidjson::kNullType };
                    }
                }
            };

            template<typename T, ReferenceCategory cat, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<reference::Ref<T, cat>, CharT>
            {
                inline Result<Json<CharT>> operator()(const reference::Ref<T, cat>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    return serializer(*obj, doc, transfer);
                }
            };

            template<typename T, typename U, CharType CharT>
                requires SerializableToJson<T, CharT> && SerializableToJson<U, CharT>
            struct ToJsonValue<std::pair<T, U>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::pair<T, U>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer1{};
                    OSPF_TRY_GET(sub_json1, serializer1(obj.first, doc, transfer));
                    json.PushBack(sub_json1.Move(), doc.GetAllocator());
                    static const ToJsonValue<OriginType<U>, CharT> serializer2{};
                    OSPF_TRY_GET(sub_json2, serializer2(obj.second, doc, transfer));
                    json.PushBack(sub_json2.Move(), doc.GetAllocator());
                    return std::move(json);
                }
            };

            template<typename... Ts, CharType CharT>
            struct ToJsonValue<std::tuple<Ts...>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::tuple<Ts...>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    OSPF_TRY_EXEC(serialize<0_uz>(json, obj, doc, transfer));
                    return std::move(json);
                }

            private:
                template<usize i>
                inline static Try<> serialize(Json<CharT>& json, const std::tuple<Ts...>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) noexcept
                {
                    if constexpr (i == VariableTypeList<Ts...>::length)
                    {
                        return succeed;
                    }
                    else
                    {
                        using ValueType = OriginType<decltype(std::get<i>(obj))>;
                        static_assert(SerializableToJson<ValueType, CharT>);
                        static const ToJsonValue<ValueType, CharT> serializer{};
                        OSPF_TRY_GET(sub_json, serializer(std::get<i>(obj), doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                        return serialize<i + 1_uz>(json, obj, doc, transfer);
                    }
                }
            };

            template<typename... Ts, CharType CharT>
            struct ToJsonValue<std::variant<Ts...>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::variant<Ts...>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kObjectType };
                    static const auto index_key = transfer.has_value() ? (*transfer)("index") : boost::locale::conv::to_utf<CharT>("index", std::locale{});
                    static const auto value_key = transfer.has_value() ? (*transfer)("value") : boost::locale::conv::to_utf<CharT>("value", std::locale{});
                    json.AddMember(rapidjson::StringRef(index_key.data()), Json<CharT>{ obj.index() }, doc.GetAllocator());
                    OSPF_TRY_EXEC(std::visit([&json, &doc, &transfer](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static_assert(SerializableToJson<ValueType, CharT>);
                            static const ToJsonValue<ValueType, CharT> serializer{};
                            OSPF_TRY_GET(sub_json, serializer(this_value, doc, transfer));
                            json.AddMember(rapidjson::StringRef(value_key.data()), sub_json.Move(), doc.GetAllocator());
                            return succeed;
                        }, obj));
                    return std::move(json);
                }
            };

            template<typename T, typename U, CharType CharT>
                requires SerializableToJson<T, CharT> && SerializableToJson<U, CharT>
            struct ToJsonValue<Either<T, U>, CharT>
            {
                inline Result<Json<CharT>> operator()(const Either<T, U>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kObjectType };
                    static const auto index_key = transfer.has_value() ? (*transfer)("index") : boost::locale::conv::to_utf<CharT>("index", std::locale{});
                    static const auto value_key = transfer.has_value() ? (*transfer)("value") : boost::locale::conv::to_utf<CharT>("value", std::locale{});
                    json.AddMember(rapidjson::StringRef(index_key.data()), Json<CharT>{ obj.is_left() ? 0_uz : 1_uz }, doc.GetAllocator());
                    OSPF_TRY_EXEC(std::visit([&json, &doc, &transfer](const auto& this_value)
                        {
                            using ValueType = OriginType<decltype(this_value)>;
                            static const ToJsonValue<ValueType, CharT> serializer{};
                            OSPF_TRY_GET(sub_json, serializer(this_value, doc, transfer));
                            json.AddMember(rapidjson::StringRef(value_key.data()), sub_json.Move(), doc.GetAllocator());
                            return succeed;
                        }, obj));
                    return std::move(json);
                }
            };

            template<typename T, ReferenceCategory cat, CopyOnWrite cow, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<ValOrRef<T, cat, cow>, CharT>
            {
                inline Result<Json<CharT>> operator()(const ValOrRef<T, cat, cow>& obj, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    return serializer(*obj, doc, transfer);
                }
            };

            template<typename T, usize len, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<std::array<T, len>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::array<T, len>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<std::vector<T>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::vector<T>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<typename T, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<std::deque<T>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::deque<T>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<typename T, usize len, CharType CharT>
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<std::span<const T, len>, CharT>
            {
                inline Result<Json<CharT>> operator()(const std::span<const T, len>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                usize len,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<optional_array::StaticOptionalArray<T, len, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const optional_array::StaticOptionalArray<T, len, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        if (obj.has_value())
                        {
                            OSPF_TRY_GET(sub_json, serializer(*obj, doc, transfer));
                            json.PushBack(sub_json.Move(), doc.GetAllocator());
                        }
                        else
                        {
                            json.PushBack(Json<CharT>{ rapidjson::kNullType }, doc.GetAllocator());
                        }
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<optional_array::DynamicOptionalArray<T, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const optional_array::DynamicOptionalArray<T, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        if (obj.has_value())
                        {
                            OSPF_TRY_GET(sub_json, serializer(*obj, doc, transfer));
                            json.PushBack(sub_json.Move(), doc.GetAllocator());
                        }
                        else
                        {
                            json.PushBack(Json<CharT>{ rapidjson::kNullType }, doc.GetAllocator());
                        }
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                usize len,
                PointerCategory cat,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<pointer_array::StaticPointerArray<T, len, cat, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const pointer_array::StaticPointerArray<T, len, cat, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        if (obj != nullptr)
                        {
                            OSPF_TRY_GET(sub_json, serializer(*obj, doc, transfer));
                            json.PushBack(sub_json.Move(), doc.GetAllocator());
                        }
                        else
                        {
                            json.PushBack(Json<CharT>{ rapidjson::kNullType }, doc.GetAllocator());
                        }
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                PointerCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<pointer_array::DynamicPointerArray<T, cat, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const pointer_array::DynamicPointerArray<T, cat, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        if (obj != nullptr)
                        {
                            OSPF_TRY_GET(sub_json, serializer(*obj, doc, transfer));
                            json.PushBack(sub_json.Move(), doc.GetAllocator());
                        }
                        else
                        {
                            json.PushBack(Json<CharT>{ rapidjson::kNullType }, doc.GetAllocator());
                        }
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                usize len,
                ReferenceCategory cat,
                template<typename, usize> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<reference_array::StaticReferenceArray<T, len, cat, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const reference_array::StaticReferenceArray<T, len, cat, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<reference_array::DynamicReferenceArray<T, cat, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const reference_array::DynamicReferenceArray<T, cat, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<tagged_map::TaggedMap<T, E, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const tagged_map::TaggedMap<T, E, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
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
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const value_or_reference_array::StaticValueOrReferenceArray<T, len, cat, cow, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<
                typename T,
                ReferenceCategory cat,
                CopyOnWrite cow,
                template<typename> class C,
                CharType CharT
            >
                requires SerializableToJson<T, CharT>
            struct ToJsonValue<value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>, CharT>
            {
                inline Result<Json<CharT>> operator()(const value_or_reference_array::DynamicValueOrReferenceArray<T, cat, cow, C>& objs, Document<CharT>& doc, const std::optional<NameTransfer<CharT>>& transfer) const noexcept
                {
                    Json<CharT> json{ rapidjson::kArrayType };
                    static const ToJsonValue<OriginType<T>, CharT> serializer{};
                    for (const auto& obj : objs)
                    {
                        OSPF_TRY_GET(sub_json, serializer(obj, doc, transfer));
                        json.PushBack(sub_json.Move(), doc.GetAllocator());
                    }
                    return std::move(json);
                }
            };

            template<>
            struct ToJsonValue<bool, char>
            {
                inline Result<rapidjson::Value> operator()(const bool value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_bool(value, doc);
                }
            };

            template<>
            struct ToJsonValue<u8, char>
            {
                inline Result<rapidjson::Value> operator()(const u8 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_u8(value, doc);
                }
            };

            template<>
            struct ToJsonValue<i8, char>
            {
                inline Result<rapidjson::Value> operator()(const i8 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_i8(value, doc);
                }
            };

            template<>
            struct ToJsonValue<u16, char>
            {
                inline Result<rapidjson::Value> operator()(const u16 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_u16(value, doc);
                }
            };

            template<>
            struct ToJsonValue<i16, char>
            {
                inline Result<rapidjson::Value> operator()(const i16 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_i16(value, doc);
                }
            };

            template<>
            struct ToJsonValue<u32, char>
            {
                inline Result<rapidjson::Value> operator()(const u32 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_u32(value, doc);
                }
            };

            template<>
            struct ToJsonValue<i32, char>
            {
                inline Result<rapidjson::Value> operator()(const i32 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_i32(value, doc);
                }
            };

            template<>
            struct ToJsonValue<u64, char>
            {
                inline Result<rapidjson::Value> operator()(const u64 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_u64(value, doc);
                }
            };

            template<>
            struct ToJsonValue<i64, char>
            {
                inline Result<rapidjson::Value> operator()(const i64 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_i64(value, doc);
                }
            };

            template<>
            struct ToJsonValue<f32, char>
            {
                inline Result<rapidjson::Value> operator()(const f32 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_f32(value, doc);
                }
            };

            template<>
            struct ToJsonValue<f64, char>
            {
                inline Result<rapidjson::Value> operator()(const f64 value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_f64(value, doc);
                }
            };

            template<>
            struct ToJsonValue<std::string, char>
            {
                inline Result<rapidjson::Value> operator()(const std::string& value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_string(value, doc);
                }
            };

            template<>
            struct ToJsonValue<std::string_view, char>
            {
                inline Result<rapidjson::Value> operator()(const std::string& value, rapidjson::Document& doc, const std::optional<NameTransfer<char>>& transfer) const noexcept
                {
                    return from_string_view(value, doc);
                }
            };
        };
    };
};
