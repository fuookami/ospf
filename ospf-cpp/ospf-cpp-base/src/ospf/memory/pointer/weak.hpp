#pragma once

#include <ospf/memory/pointer/shared.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace pointer
        {
            template<typename T>
            class Ptr<T, PointerCategory::Weak>
                : public PtrImpl<T, Ptr<T, PointerCategory::Weak>>
            {
                template<typename T, PointerCategory>
                friend class Ptr;

                template<typename T, ReferenceCategory>
                friend class Ref;

            private:
                using Impl = PtrImpl<T, Ptr<T, PointerCategory::Weak>>;

            public:
                using typename Impl::PtrType;
                using typename Impl::CPtrType;
                using typename Impl::RefType;
                using typename Impl::CRefType;
                using typename Impl::DeleterType;
                using typename Impl::DefaultDeleterType;
                using SharedPtrType = std::shared_ptr<T>;
                using WeakPtrType = std::weak_ptr<T>;

            public:
                Ptr(void) noexcept
                    : _ptr(nullptr) {}

                Ptr(const std::nullptr_t _) noexcept
                    : _ptr(nullptr) {}

                Ptr(WeakPtrType ptr) noexcept
                    : _ptr(move<WeakPtrType>(ptr)) {}

                Ptr(const SharedPtrType& ptr) noexcept
                    : _ptr(ptr) {}

                Ptr(const std::shared_ptr<const T>& ptr) noexcept
                    : _ptr(std::const_pointer_cast<T>(ptr)) {}

                Ptr(const Ptr<T, PointerCategory::Shared>& ptr) noexcept
                    : _ptr(ptr._ptr) {}

            public:
                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(std::weak_ptr<U> ptr) noexcept
                    : _ptr(move<std::weak_ptr<U>>(ptr)) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::weak_ptr<const U> ptr) noexcept
                    : _ptr(std::const_pointer_cast<U>(ptr.lock())) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::shared_ptr<U>& ptr) noexcept
                    : _ptr(ptr) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::shared_ptr<const U>& ptr) noexcept
                    : _ptr(std::const_pointer_cast<U>(ptr)) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const Ptr<U, PointerCategory::Shared>& ptr) noexcept
                    : _ptr(ptr._ptr) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const Ptr<U, PointerCategory::Weak>& ptr) noexcept
                    : _ptr(ptr._ptr) {}

            public:
                Ptr(const Ptr& ano) = default;
                Ptr(Ptr&& ano) noexcept = default;
                Ptr& operator=(const Ptr& rhs) = default;
                Ptr& operator=(Ptr&& rhs) noexcept = default;
                ~Ptr(void) noexcept = default;

            public:
                inline decltype(auto) lock(void) const noexcept
                {
                    return Ptr<T, PointerCategory::Shared>{ _ptr.lock() };
                }

                inline usize use_count(void) const noexcept
                {
                    return static_cast<usize>(_ptr.use_count());
                }

                inline const bool expired(void) const noexcept
                {
                    return _ptr.expired();
                }

                inline void reset(const std::nullptr_t _ = nullptr) noexcept
                {
                    _ptr.reset();
                }

                inline void swap(Ptr& ano) noexcept
                {
                    std::swap(_ptr, ano._ptr);
                }

            OSPF_CRTP_PERMISSION:
                inline const PtrType OSPF_CRTP_FUNCTION(get_ptr)(void) noexcept
                {
                    return _ptr.lock().get();
                }

                inline const CPtrType OSPF_CRTP_FUNCTION(get_cptr)(void) const noexcept
                {
                    return const_cast<CPtrType>(_ptr.lock().get());
                }

            private:
                WeakPtrType _ptr;
            };
        };
    };
};
