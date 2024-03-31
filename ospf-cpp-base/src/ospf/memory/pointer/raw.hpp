#pragma once

#include <ospf/memory/pointer/impl.hpp>
#include <ospf/memory/pointer/category.hpp>
#include <ospf/memory/pointer/unique.hpp>
#include <ospf/memory/pointer/shared.hpp>
#include <ospf/memory/pointer/weak.hpp>
#include <ospf/memory/reference/category.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace pointer
        {
            template<typename T>
            class Ptr<T, PointerCategory::Raw>
                : public PtrImpl<T, Ptr<T, PointerCategory::Raw>>
            {
                template<typename U, ReferenceCategory category>
                friend class Ref;

            private:
                using Impl = PtrImpl<T, Ptr<T, PointerCategory::Raw>>;

            public:
                using typename Impl::PtrType;
                using typename Impl::CPtrType;
                using typename Impl::RefType;
                using typename Impl::CRefType;

            public:
                constexpr Ptr(void) noexcept
                    : _from(PointerCategory::Raw), _ptr(nullptr) {}

                constexpr Ptr(std::nullptr_t _) noexcept
                    : _from(PointerCategory::Raw), _ptr(nullptr) {}

                Ptr(const PtrType ptr) noexcept
                    : _from(PointerCategory::Raw), _ptr(ptr) {}

                Ptr(const CPtrType cptr) noexcept
                    : _from(PointerCategory::Raw), _ptr(const_cast<PtrType>(cptr)) {}

                Ptr(CRefType cref) noexcept
                    : _from(PointerCategory::Raw), _ptr(const_cast<PtrType>(&cref)) {}

            public:
                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const ospf::PtrType<U> ptr) noexcept
                    : Ptr(static_cast<PtrType>(ptr)) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const ospf::CPtrType<U> ptr) noexcept
                    : Ptr(static_cast<CPtrType>(ptr)) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(ospf::CLRefType<U> cref) noexcept
                    : Ptr(static_cast<CPtrType>(&cref)) {}

            public:
                template<typename D>
                explicit Ptr(const std::unique_ptr<T, D>& ptr) noexcept
                    : _from(PointerCategory::Unique), _ptr(ptr.get()) {}

                template<typename D>
                explicit Ptr(const std::unique_ptr<const T, D>& ptr) noexcept
                    : _from(PointerCategory::Unique), _ptr(const_cast<PtrType>(ptr.get())) {}

                explicit Ptr(const std::shared_ptr<T>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(ptr.get()) {}

                explicit Ptr(const std::shared_ptr<const T>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(const_cast<PtrType>(ptr.get())) {}

                explicit Ptr(const std::weak_ptr<T> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(ptr.lock().get()) {}

                explicit Ptr(const std::weak_ptr<const T> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(const_cast<PtrType>(ptr.lock().get())) {}

            public:
                template<typename U, typename D>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::unique_ptr<U, D>& ptr) noexcept
                    : _from(PointerCategory::Unique), _ptr(static_cast<PtrType>(ptr.get())) {}

                template<typename U, typename D>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const std::unique_ptr<const U, D>& ptr) noexcept
                    : _from(PointerCategory::Unique), _ptr(const_cast<PtrType>(static_cast<CPtrType>(ptr.get()))) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::shared_ptr<U>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(static_cast<PtrType>(ptr.get())) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const std::shared_ptr<const U>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(const_cast<PtrType>(static_cast<CPtrType>(ptr.get()))) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const std::weak_ptr<U> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(static_cast<PtrType>(ptr.lock().get())) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const std::weak_ptr<const U> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(const_cast<PtrType>(static_cast<CPtrType>(ptr.lock().get()))) {}

            public:
                explicit Ptr(const Ptr<T, PointerCategory::Unique>& ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                    : _from(PointerCategory::Unique), _ptr(ptr._ptr.get()) 
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker = std::make_unique<UniquePtrLocker>{ ptr };
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const Ptr<U, PointerCategory::Unique>& ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                    : _from(PointerCategory::Unique), _ptr(static_cast<PtrType>(ptr._ptr.get()))
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker = std::make_unique<UniquePtrLocker>{ ptr };
#endif
                }

            public:
                explicit Ptr(const Ptr<T, PointerCategory::Shared>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(ptr._ptr.get()) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const Ptr<U, PointerCategory::Shared>& ptr) noexcept
                    : _from(PointerCategory::Shared), _ptr(static_cast<PtrType>(ptr._ptr.get())) {}

            public:
                explicit Ptr(const Ptr<T, PointerCategory::Weak> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(ptr._ptr.lock().get()) {}

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const Ptr<U, PointerCategory::Weak> ptr) noexcept
                    : _from(PointerCategory::Weak), _ptr(static_cast<PtrType>(ptr._ptr.lock().get())) {}

            public:
                Ptr(const Ptr& ano) = default;
                Ptr(Ptr&& ano) noexcept = default;
                Ptr& operator=(const Ptr& rhs) = default;
                Ptr& operator=(Ptr&& rhs) noexcept = default;
                ~Ptr(void) noexcept = default;

            public:
                inline const PointerCategory from(void) const noexcept
                {
                    return _from;
                }

            public:
                inline void reset(const std::nullptr_t _ = nullptr) noexcept
                {
                    _from = PointerCategory::Raw;
                    _ptr = nullptr;
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const PtrType ptr) noexcept
                {
                    _from = PointerCategory::Raw;
                    _ptr = ptr;
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const CPtrType cptr) noexcept
                {
                    _from = PointerCategory::Raw;
                    _ptr = const_cast<PtrType>(cptr);
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(CRefType cref) noexcept
                {
                    _from = PointerCategory::Raw;
                    _ptr = const_cast<PtrType>(&cref);
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const ospf::PtrType<U> ptr) noexcept
                {
                    reset(static_cast<PtrType>(ptr));
                }

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                inline void reset(const ospf::CPtrType<U> cptr) noexcept
                {
                    reset(static_cast<CPtrType>(cptr));
                }

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                inline void reset(ospf::CLRefType<U> cref) noexcept
                {
                    reset(static_cast<CPtrType>(&cref));
                }

                template<typename D>
                inline void reset(const std::unique_ptr<T, D>& ptr) noexcept
                {
                    _from = PointerCategory::Unique;
                    _ptr = ptr.get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename D>
                inline void reset(const std::unique_ptr<const T, D>& ptr) noexcept
                {
                    _from = PointerCategory::Unique;
                    _ptr = const_cast<PtrType>(ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const std::shared_ptr<T>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = ptr.get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const std::shared_ptr<const T>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = const_cast<PtrType>(ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const std::weak_ptr<T> ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = ptr.lock().get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const std::weak_ptr<const T> ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = const_cast<PtrType>(ptr.lock().get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U, typename D>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const std::unique_ptr<U, D>& ptr) noexcept
                {
                    _from = PointerCategory::Unique;
                    _ptr = static_cast<PtrType>(ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U, typename D>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                inline void reset(const std::unique_ptr<const U, D>& ptr) noexcept
                {
                    _from = PointerCategory::Unique;
                    _ptr = const_cast<PtrType>(static_cast<CPtrType>(ptr.get()));
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const std::shared_ptr<U>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = static_cast<PtrType>(ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                inline void reset(const std::shared_ptr<const U>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = const_cast<PtrType>(static_cast<CPtrType>(ptr.get()));
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const std::weak_ptr<U> ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = static_cast<PtrType>(ptr.lock().get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                inline void reset(const std::weak_ptr<const U> ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = const_cast<PtrType>(static_cast<CPtrType>(ptr.lock().get()));
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const Ptr<T, PointerCategory::Unique>& ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
                    _from = PointerCategory::Unique;
                    _ptr = ptr._ptr.get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset(new UniquePtrLocker{ ptr });
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const Ptr<U, PointerCategory::Unique>& ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
                    _from = PointerCategory::Unique;
                    _ptr = static_cast<PtrType>(ptr._ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset(new UniquePtrLocker{ ptr });
#endif
                }

                inline void reset(const Ptr<T, PointerCategory::Shared>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = ptr._ptr.get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const Ptr<U, PointerCategory::Shared>& ptr) noexcept
                {
                    _from = PointerCategory::Shared;
                    _ptr = static_cast<PtrType>(ptr._ptr.get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void reset(const Ptr<T, PointerCategory::Weak>& ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = ptr._ptr.lock().get();
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const Ptr<U, PointerCategory::Weak>& ptr) noexcept
                {
                    _from = PointerCategory::Weak;
                    _ptr = static_cast<PtrType>(ptr._ptr.lock().get());
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _locker.reset();
#endif
                }

                inline void swap(Ptr& ano)
                {
                    std::swap(_from, ano._from);
                    std::swap(_ptr, ano._ptr);
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    std::swap(_locker, anp._locker);
#endif
                }

            OSPF_CRTP_PERMISSION:
                inline const PtrType OSPF_CRTP_FUNCTION(get_ptr)(void) noexcept
                {
                    return _ptr;
                }

                inline const CPtrType OSPF_CRTP_FUNCTION(get_cptr)(void) const noexcept
                {
                    return const_cast<CPtrType>(_ptr);
                }

            private:
                PointerCategory _from;
                PtrType _ptr;
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                std::unique<UniquePtrLocker> _locker;
#endif
            };
        };
    };
};
