#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/meta_programming/crtp.hpp>
#include <ospf/meta_programming/function_trait.hpp>
#include <ospf/meta_programming/named_type.hpp>
#include <ospf/meta_programming/type_info.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace meta_info
        {
            template<typename T, typename Self>
            class MetaItemImpl
            {
                OSPF_CRTP_IMPL

            public:
                using ParentType = OriginType<T>;

            protected:
                constexpr MetaItemImpl(const std::string_view key)
                    : _key(key) {}
            public:
                constexpr MetaItemImpl(const MetaItemImpl& ano) = default;
                constexpr MetaItemImpl(MetaItemImpl&& ano) noexcept = default;
                constexpr MetaItemImpl& operator=(const MetaItemImpl& rhs) = default;
                constexpr MetaItemImpl& operator=(MetaItemImpl&& rhs) noexcept = default;
                constexpr ~MetaItemImpl(void) noexcept = default;

            public:
                inline constexpr const std::string_view key(void) const noexcept
                {
                    return _key;
                }

            public:
                inline constexpr const std::type_index index(void) const noexcept
                {
                    return TypeInfo<OriginType<decltype(value(std::declval<ParentType>()))>>::index();
                }

                inline constexpr decltype(auto) value(CLRefType<ParentType> obj) const noexcept
                {
                    return Trait::get_const_value(self(), obj);
                }

                template<typename P>
                inline constexpr decltype(auto) value(CLRefType<NamedType<ParentType, P>> obj) const noexcept
                {
                    return Trait::get_const_value(self(), obj.unwrap());
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr decltype(auto) is_writable(const Self& self, CLRefType<ParentType> obj) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(is_writable);
                        return (self.*get_impl)(obj);
                    }

                    inline static constexpr decltype(auto) get_const_value(const Self& self, CLRefType<ParentType> obj) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_value);
                        return (self.*get_impl)(obj);
                    }
                };

            private:
                std::string_view _key;
            };

            template<typename T, typename P>
                requires requires (const T& obj, const P& ptr)
            {
                { obj.*ptr } -> DecayNotSameAs<void>;
            }
            class FieldItem
                : public MetaItemImpl<OriginType<T>, FieldItem<T, P>>
            {
                using Impl = MetaItemImpl<OriginType<T>, FieldItem>;

            public:
                using typename Impl::ParentType;
                using PtrType = OriginType<P>;
                using ValueType = OriginType<decltype(std::declval<ParentType>().*std::declval<PtrType>())>;

            public:
                constexpr FieldItem(const std::string_view key, const PtrType ptr)
                    : Impl(key), _ptr(ptr) {}
                constexpr FieldItem(const FieldItem& ano) = default;
                constexpr FieldItem(FieldItem&& ano) noexcept = default;
                constexpr FieldItem& operator=(const FieldItem& rhs) = default;
                constexpr FieldItem& operator=(FieldItem&& rhs) noexcept = default;
                constexpr ~FieldItem(void) noexcept = default;

            public:
                inline static constexpr const bool writable(void) noexcept
                {
                    return true;
                }

            public:
                using Impl::value;

                inline constexpr decltype(auto) value(LRefType<ParentType> obj) const noexcept
                {
                    return obj.*_ptr;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_const_value)(CLRefType<ParentType> obj) const noexcept
                {
                    return obj.*_ptr;
                }

            private:
                PtrType _ptr;
            };

            template<typename T, typename P>
                requires requires (const T& obj, const P& ptr)
            {
                { (obj.*ptr)() } -> DecayNotSameAs<void>;
            }
            class AttributeItem
                : public MetaItemImpl<OriginType<T>, AttributeItem<T, P>>
            {
                using Impl = MetaItemImpl<OriginType<T>, AttributeItem>;

            public:
                using typename Impl::ParentType;
                using PtrType = OriginType<P>;
                using ValueType = OriginType<decltype((std::declval<ParentType>().*std::declval<PtrType>())())>;

            public:
                constexpr AttributeItem(const std::string_view key, const PtrType ptr)
                    : Impl(key), _ptr(ptr) {}
                constexpr AttributeItem(const AttributeItem& ano) = default;
                constexpr AttributeItem(AttributeItem&& ano) noexcept = default;
                constexpr AttributeItem& operator=(const AttributeItem& rhs) = default;
                constexpr AttributeItem& operator=(AttributeItem&& rhs) noexcept = default;
                constexpr ~AttributeItem(void) noexcept = default;

            public:
                inline static constexpr const bool writable(void) noexcept
                {
                    return false;
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr decltype(auto) OSPF_CRTP_FUNCTION(get_const_value)(CLRefType<ParentType> obj) const noexcept
                {
                    return (obj.*_ptr)();
                }

            private:
                PtrType _ptr;
            };
        };
    };
};
