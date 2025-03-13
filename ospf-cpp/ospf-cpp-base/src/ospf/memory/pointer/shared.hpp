#pragma once

#include <ospf/memory/pointer/unique.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace pointer
        {
            template<typename T>
            class Ptr<T, PointerCategory::Shared>
                : public PtrImpl<T, Ptr<T, PointerCategory::Shared>>
            {
                template<typename T, PointerCategory>
                friend class Ptr;

                template<typename T, ReferenceCategory>
                friend class Ref;

            private:
                using Impl = PtrImpl<T, Ptr<T, PointerCategory::Shared>>;

            public:
                using typename Impl::PtrType;
                using typename Impl::CPtrType;
                using typename Impl::RefType;
                using typename Impl::CRefType;
                using typename Impl::DeleterType;
                using typename Impl::DefaultDeleterType;
                using SharedPtrType = std::shared_ptr<T>;

            public:
                constexpr Ptr(void) noexcept
                    : _ptr(nullptr) {}
                
                constexpr Ptr(const std::nullptr_t _) noexcept
                    : _ptr(nullptr) {}

                Ptr(SharedPtrType ptr) noexcept
                    : _ptr(move<SharedPtrType>(ptr)) {}

                Ptr(const std::shared_ptr<const T>& ptr) noexcept
                    : _ptr(std::const_pointer_cast<T>(ptr)) {}

                Ptr(const PtrType ptr) noexcept
                    : _ptr(SharedPtrType{ ptr, DefaultDeleterType{} }) {}

                Ptr(const CPtrType cptr) noexcept
                    : _ptr(SharedPtrType{ const_cast<PtrType>(cptr), DefaultDeleterType{} }) {}

                template<typename D>
                explicit Ptr(const PtrType ptr, D&& deletor) noexcept
                    : _ptr(SharedPtrType{ ptr, std::forward<D>(deletor) }) {}

                template<typename D>
                explicit Ptr(const CPtrType cptr, D&& deletor) noexcept
                    : _ptr(SharedPtrType{ cptr, std::forward<D>(deletor) }) {}

            public:
                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(std::shared_ptr<U> ptr) noexcept
                    : _ptr(ptr) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::shared_ptr<const U>& ptr) noexcept
                    : _ptr(std::dynamic_pointer_cast<T>(std::const_pointer_cast<U>(ptr))) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(Ptr<U, PointerCategory::Shared> ptr) noexcept
                    : _ptr(move<std::shared_ptr<U>>(ptr._ptr)) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const ospf::PtrType<U> ptr) noexcept
                    : _ptr(SharedPtrType{ static_cast<PtrType>(ptr), DefaultDeleterType{} }) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const ospf::CPtrType<U> cptr) noexcept
                    : _ptr(SharedPtrType{ const_cast<PtrType>(static_cast<CPtrType>(cptr)), DefaultDeleterType{} }) {}

                template<typename U, typename D>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const ospf::PtrType<U> ptr, D&& deletor) noexcept
                    : _ptr(SharedPtrType{ static_cast<PtrType>(ptr), std::forward<D>(deletor) }) {}

                template<typename U, typename D>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const ospf::CPtrType<U> cptr, D&& deletor) noexcept
                    : _ptr(SharedPtrType{ const_cast<PtrType>(static_cast<CPtrType>(cptr)), std::forward<D>(deletor) }) {}

            public:
                Ptr(const Ptr& ano) = default;
                Ptr(Ptr&& ano) noexcept = default;
                Ptr& operator=(const Ptr& rhs) = default;
                Ptr& operator=(Ptr&& rhs) noexcept = default;
                ~Ptr(void) noexcept = default;

            public:
                inline usize use_count(void) const noexcept
                {
                    return static_cast<usize>(_ptr.use_count());
                }

                inline const bool unique(void) const noexcept
                {
                    return use_count() == 1_uz;
                }

                inline void reset(const std::nullptr_t _ = nullptr) noexcept
                {
                    _ptr.reset();
                }

                inline void reset(const PtrType ptr) noexcept
                {
                    _ptr.reset(ptr);
                }

                inline void reset(const CPtrType cptr) noexcept
                {
                    _ptr.reset(const_cast<PtrType>(cptr));
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const ospf::PtrType<U> ptr) noexcept
                {
                    _ptr.reset(ptr);
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const ospf::CPtrType<U> cptr) noexcept
                {
                    _ptr.reset(const_cast<ospf::PtrType<U>>(cptr));
                }

                inline void swap(Ptr& ano) noexcept
                {
                    std::swap(_ptr, ano._ptr);
                }

            OSPF_CRTP_PERMISSION:
                inline const PtrType OSPF_CRTP_FUNCTION(get_ptr)(void) noexcept
                {
                    return _ptr.get();
                }

                inline const CPtrType OSPF_CRTP_FUNCTION(get_cptr)(void) const noexcept
                {
                    return const_cast<CPtrType>(_ptr.get());
                }

            private:
                SharedPtrType _ptr;
            };

            template<typename T, typename... Args>
                requires std::is_constructible_v<T, Args...>
            inline decltype(auto) make_shared(Args&&... args)
            {
                return pointer::Ptr<T, PointerCategory::Shared>{ new T{ std::forward<Args>(args)... } };
            }

            template<typename T, typename U, typename... Args>
                requires std::convertible_to<PtrType<U>, PtrType<T>> && std::is_constructible_v<U, Args...>
            inline decltype(auto) make_base_shared(Args&&... args)
            {
                return pointer::Ptr<T, PointerCategory::Shared>{ static_cast<T*>(new U{ std::forward<Args>(args)... }) };
            }
        };
    };
};
