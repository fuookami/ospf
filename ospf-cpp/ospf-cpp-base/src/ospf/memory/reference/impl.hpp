#pragma once

#include <ospf/type_family.hpp>
#include <ospf/concepts/with_tag.hpp>
#include <ospf/meta_programming/crtp.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace reference
        {
            template<typename T, typename Self>
            class RefImpl
            {
                OSPF_CRTP_IMPL

            public:
                using PtrType = PtrType<T>;
                using CPtrType = CPtrType<T>;
                using RefType = LRefType<T>;
                using CRefType = CLRefType<T>;

            protected:
                constexpr RefImpl(void) noexcept = default;
            public:
                constexpr RefImpl(const RefImpl& ano) = default;
                constexpr RefImpl(RefImpl&& ano) noexcept = default;
                constexpr RefImpl& operator=(const RefImpl& rhs) = default;
                constexpr RefImpl& operator=(RefImpl&& rhs) noexcept = default;
                constexpr ~RefImpl(void) noexcept = default;

            public:
                inline constexpr const PtrType operator&(void) noexcept
                {
                    return &ref();
                }

                inline constexpr const CPtrType operator&(void) const noexcept
                {
                    return &cref();
                }

                inline constexpr const PtrType operator->(void) noexcept
                {
                    return &ref();
                }

                inline constexpr const CPtrType operator->(void) const noexcept
                {
                    return &cref();
                }

                inline constexpr RefType operator*(void) noexcept
                {
                    return ref();
                }

                inline constexpr CRefType operator*(void) const noexcept
                {
                    return cref();
                }

            public:
                inline constexpr operator RefType(void) noexcept
                {
                    return ref();
                }

                inline constexpr operator CRefType(void) const noexcept
                {
                    return cref();
                }

            public:
                template<typename U, typename R>
                inline constexpr const bool operator==(const RefImpl<U, R>& rhs) const noexcept
                {
                    return &cref() == &rhs.cref();
                }

                template<typename U, typename R>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator==(const RefImpl<U, R>& rhs) const noexcept
                {
                    return &cref() == &rhs.cref() || cref() == rhs.cref();
                }

                template<typename U, typename R>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator==(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() == rhs.cref();
                }

                template<typename U, typename R>
                inline constexpr const bool operator!=(const RefImpl<U, R>& rhs) const noexcept
                {
                    return &cref() != &rhs.cref();
                }

                template<typename U, typename R>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator!=(const RefImpl<U, R>& rhs) const noexcept
                {
                    return &cref() != &rhs.cref() && cref() != rhs.cref();
                }

                template<typename U, typename R>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator!=(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() != rhs.cref();
                }

                template<typename U>
                inline constexpr const bool operator==(const U& rhs) const noexcept
                {
                    return &cref() == &rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator==(const U& rhs) const noexcept
                {
                    return &cref() == &rhs || cref() == rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator==(const U& rhs) const noexcept
                {
                    return cref() == rhs;
                }

                template<typename U>
                inline constexpr const bool operator!=(const U& rhs) const noexcept
                {
                    return &cref() != &rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator!=(const U& rhs) const noexcept
                {
                    return &cref() != &rhs && cref() != rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator!=(const U& rhs) const noexcept
                {
                    return cref() != rhs;
                }

                template<typename U>
                inline constexpr const bool operator==(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && &cref() == &*rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator==(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && (&cref() == &*rhs || cref() == *rhs);
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator==(const std::optional<U>& rhs) const
                {
                    return cref() == *rhs;
                }

                template<typename U>
                inline constexpr const bool operator!=(const std::optional<U>& rhs) const noexcept
                {
                    return !static_cast<bool>(rhs) || &cref() != &*rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator!=(const std::optional<U>& rhs) const noexcept
                {
                    return !static_cast<bool>(rhs) || (&cref() != &*rhs && cref() != *rhs);
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator!=(const std::optional<U>& rhs) const
                {
                    return cref() != *rhs;
                }

            public:
                template<typename U, typename R>
                inline constexpr decltype(auto) operator<(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() < rhs.cref();
                }

                template<typename U, typename R>
                inline constexpr decltype(auto) operator<=(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() <= rhs.cref();
                }

                template<typename U, typename R>
                inline constexpr decltype(auto) operator>(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() > rhs.cref();
                }

                template<typename U, typename R>
                inline constexpr decltype(auto) operator>=(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() >= rhs.cref();
                }

                template<typename U>
                inline constexpr decltype(auto) operator<(const U& rhs) const noexcept
                {
                    return cref() < rhs;
                }

                template<typename U>
                inline constexpr decltype(auto) operator<=(const U& rhs) const noexcept
                {
                    return cref() <= rhs;
                }

                template<typename U>
                inline constexpr decltype(auto) operator>(const U& rhs) const noexcept
                {
                    return cref() > rhs;
                }

                template<typename U>
                inline constexpr decltype(auto) operator>=(const U& rhs) const noexcept
                {
                    return cref() >= rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator<(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && cref() < *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator<(const std::optional<U>& rhs) const
                {
                    return cref() < *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator<=(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && cref() <= *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator<=(const std::optional<U>& rhs) const
                {
                    return cref() <= *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator>(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && cref() > *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator>(const std::optional<U>& rhs) const
                {
                    return cref() > *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs >= rhs } -> DecaySameAs<bool>; }
                inline constexpr const bool operator>=(const std::optional<U>& rhs) const noexcept
                {
                    return static_cast<bool>(rhs) && cref() >= *rhs;
                }

                template<typename U>
                    requires requires (const T& lhs, const U& rhs) { { lhs >= rhs } -> DecayNotSameAs<void, bool>; }
                inline constexpr decltype(auto) operator>=(const std::optional<U>& rhs) const
                {
                    return cref() >= *rhs;
                }

            public:
                template<typename U, typename R>
                inline constexpr decltype(auto) operator<=>(const RefImpl<U, R>& rhs) const noexcept
                {
                    return cref() <=> rhs.cref();
                }

                template<typename U>
                inline constexpr decltype(auto) operator<=>(const U& rhs) const noexcept
                {
                    return cref() <=> rhs;
                }

                template<typename U>
                inline constexpr RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<U>& rhs) const noexcept
                {
                    if (static_cast<bool>(rhs))
                    {
                        return cref() <=> *rhs;
                    }
                    else
                    {
                        return true <=> static_cast<bool>(rhs);
                    }
                }

            protected:
                inline constexpr RefType ref(void) noexcept
                {
                    return Trait::get_ref(self());
                }

                inline constexpr CRefType cref(void) const noexcept
                {
                    return Trait::get_cref(self());
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr RefType get_ref(Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_ref);
                        return (self.*get_impl)();
                    }

                    inline static constexpr CRefType get_cref(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_cref);
                        return (self.*get_impl)();
                    }
                };
            };
        };
    };
};

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator==(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs == *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator!=(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs != *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator==(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs == *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs == rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator==(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs == *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator!=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return !static_cast<bool>(lhs) || *lhs != *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs != rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator!=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs != *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator<(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs < *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator<=(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs <= *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator>(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs > *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator>=(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs >= *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator<(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs < *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs < rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator<(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs < *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator<=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs <= *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs <= rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator<=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs <= *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator>(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs > *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator>(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs > *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs >= rhs } -> ospf::DecaySameAs<bool>; }
inline constexpr const bool operator>=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return static_cast<bool>(lhs) && *lhs >= *rhs;
}

template<typename T, typename U, typename R>
    requires requires (const T& lhs, const U& rhs) { { lhs > rhs } -> ospf::DecayNotSameAs<void, bool>; }
inline constexpr decltype(auto) operator>=(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs)
{
    return *lhs >= *rhs;
}

template<typename T, typename U, typename R>
inline constexpr decltype(auto) operator<=>(const T& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    return lhs <=> *rhs;
}

template<typename T, typename U, typename R>
inline constexpr ospf::RetType<std::compare_three_way_result_t<T, U>> operator<=>(const std::optional<T>& lhs, const ospf::reference::RefImpl<U, R>& rhs) noexcept
{
    if (static_cast<bool>(lhs))
    {
        return *lhs <=> *rhs;
    }
    else
    {
        return static_cast<bool>(lhs) <=> true;
    }
}

namespace std
{
    template<typename T, typename Ref>
    struct hash<ospf::reference::RefImpl<T, Ref>>
    {
        using RefType = ospf::reference::RefImpl<T, Ref>;

        inline constexpr const ospf::usize operator()(ospf::ArgCLRefType<RefType> ref) const noexcept
        {
            static const hash<T> hasher{};
            return hasher(*ref);
        }
    };

    template<typename T, typename Ref, ospf::CharType CharT>
    struct formatter<ospf::reference::RefImpl<T, Ref>, CharT> 
        : public formatter<T, CharT>
    {
        using RefType = ospf::reference::RefImpl<T, Ref>;

        template<typename FormatContext>
        inline constexpr decltype(auto) format(ospf::ArgCLRefType<RefType> ref, FormatContext& fc) const
        {
            static const formatter<ospf::OriginType<T>, CharT> _formatter{};
            return _formatter.format(*ref, fc);
        }
    };
};

namespace ospf
{
    template<typename T, typename Ref>
        requires WithTag<T>
    struct TagValue<reference::RefImpl<T, Ref>>
    {
        using Type = typename TagValue<T>::Type;
        using RefType = reference::RefImpl<T, Ref>;

        inline constexpr RetType<Type> value(ArgCLRefType<RefType> ref) const noexcept
        {
            static constexpr const TagValue<T> extractor{};
            return extractor(*ref);
        }
    };
};
