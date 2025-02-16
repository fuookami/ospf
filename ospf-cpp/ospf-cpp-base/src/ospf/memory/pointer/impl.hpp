#pragma once

#include <ospf/type_family.hpp>
#include <ospf/concepts/with_tag.hpp>
#include <ospf/meta_programming/crtp.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace pointer
        {
            template<typename T, typename Self>
            class PtrImpl;

            template<typename T, typename Self>
            class PtrImpl
            {
                OSPF_CRTP_IMPL

            public:
                using PtrType = PtrType<T>;
                using CPtrType = CPtrType<T>;
                using RefType = LRefType<T>;
                using CRefType = CLRefType<T>;

                using DeleterType = std::function<void(T*)>;
                using DefaultDeleterType = std::default_delete<T>;

            protected:
                PtrImpl(void) noexcept = default;
            public:
                PtrImpl(const PtrImpl& ano) = default;
                PtrImpl(PtrImpl&& ano) noexcept = default;
                PtrImpl& operator=(const PtrImpl& rhs) = default;
                PtrImpl& operator=(PtrImpl&& rhs) noexcept = default;
                ~PtrImpl(void) noexcept = default;

            public:
                inline const bool null(void) const noexcept
                {
                    return cptr() == nullptr;
                }

                inline const bool operator!(void) const noexcept
                {
                    return null();
                }

                inline operator const bool(void) const noexcept
                {
                    return !null();
                }

            public:
                inline const bool operator==(const std::nullptr_t _) const noexcept
                {
                    return cptr() == nullptr;
                }

                inline const bool operator!=(const std::nullptr_t _) const noexcept
                {
                    return cptr() != nullptr;
                }

                template<typename U, typename P>
                inline const bool operator==(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() == ptr.cptr();
                }

                template<typename U, typename P>
                inline const bool operator!=(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() != ptr.cptr();
                }

                template<typename U>
                inline const bool operator==(const U* const ptr) const noexcept
                {
                    return cptr() == ptr;
                }

                template<typename U>
                inline const bool operator!=(const U* const ptr) const noexcept
                {
                    return cptr() != ptr;
                }

            public:
                template<typename U, typename P>
                inline const bool operator<(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() < ptr.cptr();
                }

                template<typename U, typename P>
                inline const bool operator<=(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() <= ptr.cptr();
                }

                template<typename U, typename P>
                inline const bool operator>(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() > ptr.cptr();
                }

                template<typename U, typename P>
                inline const bool operator>=(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() >= ptr.cptr();
                }

                template<typename U>
                inline const bool operator<(const U* const ptr) const noexcept
                {
                    return cptr() < ptr;
                }

                template<typename U>
                inline const bool operator<=(const U* const ptr) const noexcept
                {
                    return cptr() <= ptr;
                }

                template<typename U>
                inline const bool operator>(const U* const ptr) const noexcept
                {
                    return cptr() > ptr;
                }

                template<typename U>
                inline const bool operator>=(const U* const ptr) const noexcept
                {
                    return cptr() >= ptr;
                }

            public:
                template<typename U, typename P>
                inline decltype(auto) operator<=>(const PtrImpl<U, P>& ptr) const noexcept
                {
                    return cptr() <=> ptr.cptr();
                }

                template<typename U>
                inline decltype(auto) operator<=>(const U* const ptr) const noexcept
                {
                    return cptr() <=> ptr;
                }

            public:
                inline const PtrType operator&(void) noexcept
                {
                    return ptr();
                }

                inline const CPtrType operator&(void) const noexcept
                {
                    return cptr();
                }

                inline const PtrType operator->(void) noexcept
                {
                    return ptr();
                }

                inline const CPtrType operator->(void) const noexcept
                {
                    return cptr();
                }

                inline RefType operator*(void)
                {
                    assert(!null());
                    return *ptr();
                }

                inline CRefType operator*(void) const
                {
                    assert(!null());
                    return *cptr();
                }

            public:
                inline operator const PtrType(void) noexcept
                {
                    return ptr();
                }

                inline operator const CPtrType(void) const noexcept
                {
                    return cptr();
                }

                inline operator const ptraddr(void) const noexcept
                {
                    return reinterpret_cast<ptraddr>(cptr());
                }

            protected:
                inline const PtrType ptr(void) noexcept
                {
                    return Trait::get_ptr(self());
                }

                inline const CPtrType cptr(void) const noexcept
                {
                    return Trait::get_cptr(self());
                }

            private:
                struct Trait : public Self
                {
                    inline static const PtrType get_ptr(Self& self) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(get_ptr);
                        return (self.*impl)();
                    }

                    inline static const CPtrType get_cptr(const Self& self) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(get_cptr);
                        return (self.*impl)();
                    }
                };
            };

            template<typename T, typename Ptr>
            class PtrImpl<T[], Ptr>
                : public PtrImpl<T, Ptr>
            {
            public:
                using typename PtrImpl<T, Ptr>::PtrType;
                using typename PtrImpl<T, Ptr>::CPtrType;
                using typename PtrImpl<T, Ptr>::RefType;
                using typename PtrImpl<T, Ptr>::CRefType;
                using DeleterType = std::function<void(T*)>;
                using DefaultDeleterType = std::default_delete<T[]>;

            protected:
                PtrImpl(void) noexcept = default;
            public:
                PtrImpl(const PtrImpl& ano) = default;
                PtrImpl(PtrImpl&& ano) noexcept = default;
                PtrImpl& operator=(const PtrImpl& rhs) = default;
                PtrImpl& operator=(PtrImpl&& rhs) noexcept = default;
                ~PtrImpl(void) noexcept = default;

            public:
                inline CRefType operator[](const usize i) const
                {
                    return this->get_cptr()[i];
                }
            };
        };
    };
};

namespace std
{
    template<typename T, typename Ptr>
    struct hash<ospf::pointer::PtrImpl<T, Ptr>>
    {
        using PtrType = ospf::pointer::PtrImpl<T, Ptr>;

        inline constexpr const ospf::usize operator()(ospf::ArgCLRefType<PtrType> ptr) const noexcept
        {
            static const auto func = hash<typename PtrType::CPtrType>{};
            return func(reinterpret_cast<typename PtrType::CPtrType>(static_cast<ospf::ptraddr>(ptr)));
        }
    };

    template<typename T, typename Ptr>
    struct formatter<ospf::pointer::PtrImpl<T, Ptr>, char> 
        : public formatter<string_view, char>
    {
        using PtrType = ospf::pointer::PtrImpl<T, Ptr>;

        template<typename FormatContext>
        inline constexpr decltype(auto) format(ospf::ArgCLRefType<PtrType> ptr, FormatContext& fc) const
        {
            if (ptr == nullptr)
            {
                static const formatter<string_view, char> _formatter{};
                return _formatter.format("null", fc);
            }
            else
            {
                static const formatter<ospf::OriginType<T>, char> _formatter{};
                return _formatter.format(*ptr, fc);
            }
        }
    };

    template<typename T, typename Ptr>
    struct formatter<ospf::pointer::PtrImpl<T, Ptr>, ospf::wchar> 
        : public formatter<wstring_view, ospf::wchar>
    {
        using PtrType = ospf::pointer::PtrImpl<T, Ptr>;

        template<typename FormatContext>
        inline constexpr decltype(auto) format(ospf::ArgCLRefType<PtrType> ptr, FormatContext& fc) const
        {
            if (ptr == nullptr)
            {
                static const formatter<wstring_view, ospf::wchar> _formatter{};
                return _formatter.format(L"null", fc);
            }
            else
            {
                static const formatter<ospf::OriginType<T>, ospf::wchar> _formatter{};
                return _formatter.format(*ptr, fc);
            }
        }
    };

    // todo: impl for different character
};

namespace ospf
{
    template<typename T, typename Ptr>
        requires WithTag<T>
    struct TagValue<pointer::PtrImpl<T, Ptr>>
    {
        using Type = typename TagValue<T>::Type;
        using PtrType = pointer::PtrImpl<T, Ptr>;

        inline constexpr RetType<Type> value(ospf::ArgCLRefType<PtrType> ptr) const
        {
            static constexpr const TagValue<T> extractor{};
            return extractor(*ptr);
        }
    };
};
