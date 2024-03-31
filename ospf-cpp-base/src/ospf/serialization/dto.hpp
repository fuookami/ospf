#pragma once

#include <ospf/meta_programming/meta_info.hpp>
#include <ospf/meta_programming/variadic_macro.hpp>

#ifndef OSPF_DTO_FIELDS
#define OSPF_DTO_FIELDS_IMPL(N, ...) OSPF_MAKE_ARG_LIST(N, OSPF_META_FIELD, __VA_ARGS__)
#define OSPF_DTO_FIELDS(...) OSPF_DTO_FIELDS_IMPL(OSPF_GET_ARG_COUNT(__VA_ARGS__), __VA_ARGS__)
#endif

#ifndef OSPF_DTO_ATTRS
#define OSPF_DTO_ATTRS_IMPL(N, ...) OSPF_MAKE_ARG_LIST(N, OSPF_META_ATTRIBUTE, __VA_ARGS__)
#define OSPF_DTO_ATTRS(...) OSPF_DTO_ATTRS_IMPL(OSPF_GET_ARG_COUNT(__VA_ARGS__), __VA_ARGS__)
#endif

#ifndef OSPF_PLANE_DTO
#define OSPF_PLANE_DTO(T, ...) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ OSPF_DTO_FIELDS(__VA_ARGS__) };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{};\
    }\
};

#define OSPF_PLANE_DTO_EX1(T, B1, ...) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ OSPF_DTO_FIELDS(__VA_ARGS__) };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{};\
    }\
};

#define OSPF_PLANE_DTO_EX2(T, B1, B2, ...) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ OSPF_DTO_FIELDS(__VA_ARGS__) };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{};\
    }\
};

#define OSPF_PLANE_DTO_EX3(T, B1, B2, B3, ...) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ OSPF_DTO_FIELDS(__VA_ARGS__) };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{};\
    }\
};

#define OSPF_PLANE_DTO_EX4(T, B1, B2, B3, B4, ...) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>, ospf::meta_info::BaseType<B4>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>, ospf::meta_info::BaseType<B4>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ OSPF_DTO_FIELDS(__VA_ARGS__) };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{};\
    }\
};

#define OSPF_DTO(T, Fs, As) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ Fs };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{ As };\
    }\
};

#define OSPF_DTO_EX1(T, B1, Fs, As) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ Fs };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{ As };\
    }\
};

#define OSPF_DTO_EX2(T, B1, B2, Fs, As) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ Fs };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{ As };\
    }\
};

#define OSPF_DTO_EX3(T, B1, B2, B3, Fs, As) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ Fs };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{ As };\
    }\
};

#define OSPF_DTO_EX4(T, B1, B2, B3, B4, Fs, As) template<>\
class ospf::meta_info::MetaInfo<T>\
    : public ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>, ospf::meta_info::BaseType<B4>>\
{\
    using Impl = ospf::meta_info::MetaInfoImpl<T, ospf::meta_info::MetaInfo<T>, ospf::meta_info::BaseType<B1>, ospf::meta_info::BaseType<B2>, ospf::meta_info::BaseType<B3>, ospf::meta_info::BaseType<B4>>;\
public:\
    using typename Impl::ParentType;\
    template<typename P>\
    using FieldItem = meta_info::FieldItem<ParentType, P>;\
    template<typename P>\
    using AttributeItem = meta_info::AttributeItem<ParentType, P>;\
public:\
    constexpr MetaInfo(void) = default;\
    constexpr MetaInfo(const MetaInfo& ano) = default;\
    constexpr MetaInfo(MetaInfo&& ano) noexcept = default;\
    constexpr MetaInfo& operator=(const MetaInfo& rhs) = default;\
    constexpr MetaInfo& operator=(MetaInfo&& rhs) noexcept = default;\
    constexpr ~MetaInfo(void) noexcept = default;\
OSPF_CRTP_PERMISSION:\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_field_list)(void) const noexcept\
    {\
        return FieldList{ Fs };\
    }\
    inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_attribute_list)(void) const noexcept\
    {\
        return AttributeList{ As };\
    }\
};

#endif
