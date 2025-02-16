#pragma once

#include <ospf/memory/pointer/impl.hpp>
#include <ospf/memory/pointer/category.hpp>
#include <ospf/memory/reference/category.hpp>
#include <memory>

#if defined(_DEBUG) && defined(OSPF_UNIQUE_PTR_CHECK)
#ifndef OSPF_UNIQUE_PTR_CHECK_NEEDED
#define OSPF_UNIQUE_PTR_CHECK_NEEDED
#endif
#endif

#ifndef OSPF_UNIQUE_PTR_NOEXCEPT
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
#define OSPF_UNIQUE_PTR_NOEXCEPT
#else
#define OSPF_UNIQUE_PTR_NOEXCEPT noexcept
#endif
#endif

namespace ospf
{
    inline namespace memory
    {
        namespace pointer
        {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
            class UniquePtrLocker
            {
                template<typename T, PointerCategory>
                friend class Ptr;

                template<typename T, ReferenceCategory>
                friend class Ref;

            private:
                using Lock = UniquePtrLockImpl;

            private:
                UniquePtrLocker(void)
                    : _lock(nullptr) {}

                template<typename T>
                UniquePtrLocker(const Ptr<T, PointerCategory::Unique>& ptr)
                    : _lock(&ptr._lock)
                {
                    if (_lock->_locked)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    _lock->_locked = true;
                }

            public:
                UniquePtrLocker(const UniqueBoxLocker& ano)
                    : _lock(nullptr)
                {
                    if (ano._lock != nullptr)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    _lock = ano._lock;
                }

                UniquePtrLocker(UniqueBoxLocker&& ano) noexcept
                    : _lock(nullptr)
                {
                    std::swap(_lock, ano._lock);
                }

                UniquePtrLocker& operator=(const UniqueBoxLocker& rhs)
                {
                    if (_lock != nullptr || rhs._lock != nullptr)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    _lock = rhs._lock;
                    return *this;
                }

                UniquePtrLocker& operator=(UniqueBoxLocker&& rhs)
                {
                    if (_lock != nullptr)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    std::swap(_lock, rhs._lock);
                    return *this;
                }

                ~UniquePtrLocker(void)
                {
                    if (_lock != nullptr)
                    {
                        _lock->_locked = false;
                    }
                }

            public:
                inline void swap(UniquePtrLocker& ano)
                {
                    std::swap(_lock, ano._lock);
                }

            private:
                Lock* const _lock;
            };

            class UniquePtrLockImpl
            {
                friend class UniquePtrLocker;

                template<typename T, PointerCategory>
                friend class Ptr;

                template<typename T, ReferenceCategory>
                friend class Ref;

            protected:
                UniquePtrLockImpl(void) noexcept
                    : _locked(false) {}
            public:
                UniquePtrLockImpl(const UniquePtrLockImpl& ano) = delete;
                UniquePtrLockImpl(UniquePtrLockImpl&& ano)
                    : _locked(false)
                {
                    if (_locked || ano._locked)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    std::swap(_locked, ano._locked);
                }
                UniquePtrLockImpl& operator=(const UniquePtrLockImpl& rhs) = delete;
                UniquePtrLockImpl& operator=(UniquePtrLockImpl&& rhs)
                {
                    swap(rhs);
                    return *this;
                }
                ~UniquePtrLockImpl(void)
                {
                    assert(!_locked);
                }

            public:
                inline void swap(UniquePtrLockImpl& ano)
                {
                    if (_locked || ano._locked)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                    std::swap(_locked, ano._locked);
                }

            private:
                volatile mutable bool _locked;
            };
#endif

            template<typename T>
            class Ptr<T, PointerCategory::Unique>
                : public PtrImpl<T, Ptr<T, PointerCategory::Unique>>
            {
                friend class UniquePtrLocker;

                template<typename T, PointerCategory>
                friend class Ptr;

                template<typename T, ReferenceCategory>
                friend class Ref;

            private:
                using Impl = PtrImpl<T, Ptr<T, PointerCategory::Unique>>;

            public:
                using typename Impl::PtrType;
                using typename Impl::CPtrType;
                using typename Impl::RefType;
                using typename Impl::CRefType;
                using typename Impl::DeleterType;
                using typename Impl::DefaultDeleterType;
                using UniquePtrType = std::unique_ptr<T, DeleterType>;

            public:
                constexpr Ptr(void) noexcept
                    : _ptr(nullptr) {}

                constexpr Ptr(const std::nullptr_t _) noexcept
                    : _ptr(nullptr) {}

                Ptr(UniquePtrType ptr) noexcept
                    : _ptr(move<UniquePtrType>(ptr)) {}

                template<typename D>
                Ptr(std::unique_ptr<T, D> ptr) noexcept
                    : _ptr(move<std::unique_ptr<T, D>>(ptr)) {}

                template<typename D>
                Ptr(std::unique_ptr<const T, D> ptr) noexcept
                    : _ptr(move<std::unique_ptr<const T, D>>(ptr)) {}

                Ptr(const PtrType ptr) noexcept
                    : _ptr(UniquePtrType{ ptr, DefaultDeleterType{} }) {}

                Ptr(const CPtrType cptr) noexcept
                    : _ptr(UniquePtrType{ const_cast<PtrType>(cptr), DefaultDeleterType{} }) {}

                template<typename D>
                explicit Ptr(const PtrType ptr, D&& deletor) noexcept
                    : _ptr(UniquePtrType{ ptr, std::forward<D>(deletor) }) {}

                template<typename D>
                explicit Ptr(const CPtrType cptr, D&& deletor) noexcept
                    : _ptr(UniquePtrType{ cptr, std::forward<D>(deletor) }) {}

            public:
                template<typename U>
                    requires DecayNotSameAs<U, T> && std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(std::unique_ptr<U> ptr) noexcept
                    : _ptr(move<std::unique_ptr<U>>(ptr)) {}

                template<typename U, typename D>
                    requires DecayNotSameAs<U, T> && std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(std::unique_ptr<U, D> ptr) noexcept
                    : _ptr(move<std::unique_ptr<U, D>>(ptr)) {}

                template<typename U, typename D>
                    requires DecayNotSameAs<U, T> && std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(std::unique_ptr<const U, D> ptr) noexcept
                    : _ptr(move<std::unique_ptr<const U, D>>(ptr)) {}

#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(Ptr<U, PointerCategory::Unique> ptr)
                    : _lock(std::move(ptr._lock)), _ptr(std::move(ptr._ptr)) {}
#else
                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(Ptr<U, PointerCategory::Unique> ptr) noexcept
                    : _ptr(std::move(ptr._ptr)) {}
#endif

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const ospf::PtrType<U> ptr) noexcept
                    : _ptr(UniquePtrType{ static_cast<PtrType>(ptr), DefaultDeleterType{} }) {}

                template<typename U>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const ospf::CPtrType<U> cptr) noexcept
                    : _ptr(UniquePtrType{ const_cast<PtrType>(static_cast<CPtrType>(cptr)), DefaultDeleterType{} }) {}

                template<typename U, typename D>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                explicit Ptr(const ospf::PtrType<U> ptr, D&& deletor) noexcept
                    : _ptr(UniquePtrType{ static_cast<PtrType>(ptr), std::forward<D>(deletor) }) {}

                template<typename U, typename D>
                    requires std::convertible_to<ospf::CPtrType<U>, CPtrType>
                explicit Ptr(const ospf::CPtrType<U> cptr, D&& deletor) noexcept
                    : _ptr(UniquePtrType{ const_cast<PtrType>(static_cast<CPtrType>(cptr)), std::forward<D>(deletor) }) {}


            public:
                Ptr(const Ptr& ano) = delete;
                Ptr(Ptr&& ano) OSPF_UNIQUE_PTR_NOEXCEPT = default;
                Ptr& operator=(const Ptr& rhs) = delete;
                Ptr& operator=(Ptr&& rhs) OSPF_UNIQUE_PTR_NOEXCEPT = default;
                ~Ptr(void) OSPF_UNIQUE_PTR_NOEXCEPT = default;

            public:
                inline const PtrType release(void) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    return _ptr.release();
                }

                inline void reset(const std::nullptr_t _ = nullptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    _ptr.reset();
                }

                inline void reset(const PtrType ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    _ptr.reset(ptr);
                }

                inline void reset(const CPtrType cptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    _ptr.reset(const_cast<PtrType>(cptr));
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const ospf::PtrType<U> ptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    _ptr.reset(ptr);
                }

                template<typename U>
                    requires std::convertible_to<ospf::PtrType<U>, PtrType>
                inline void reset(const ospf::CPtrType<U> cptr) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    _ptr.reset(const_cast<ospf::PtrType<U>>(cptr));
                }

                inline void swap(Ptr& ano) OSPF_UNIQUE_PTR_NOEXCEPT
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    _lock(ano._lock);
#endif
                    std::swap(_ptr, ano._ptr);
                }

            OSPF_CRTP_PERMISSION:
                inline const PtrType OSPF_CRTP_FUNCTION(get_ptr)(void)
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    return _ptr.get();
                }

                inline const CPtrType OSPF_CRTP_FUNCTION(get_cptr)(void) const
                {
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                    check_lock();
#endif
                    return const_cast<CPtrType>(_ptr.get());
                }

            private:
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                inline void check_lock(void) const
                {
                    if (_lock._locked)
                    {
                        throw OSPFException{ { OSPFErrCode::UniquePtrLocked } };
                    }
                }
#endif

            private:
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
                UniquePtrLockImpl _lock;
#endif
                UniquePtrType _ptr;
            };
        };

        template<typename T, typename... Args>
            requires std::is_constructible_v<T, Args...>
        inline decltype(auto) make_unique(Args&&... args)
        {
            return pointer::Ptr<T, PointerCategory::Unique>{ new T{ std::forward<Args>(args)... } };
        }

        template<typename T, typename U, typename... Args>
            requires std::convertible_to<PtrType<U>, PtrType<T>> && std::is_constructible_v<U, Args...>
        inline decltype(auto) make_base_unique(Args&&... args)
        {
            return pointer::Ptr<T, PointerCategory::Unique>{ static_cast<T*>(new U{ std::forward<Args>(args)... }) };
        }
    };
};

namespace std
{
#ifdef OSPF_UNIQUE_PTR_CHECK_NEEDED
    inline void swap(ospf::pointer::UniquePtrLocker& lhs, ospf::pointer::UniquePtrLocker& rhs)
    {
        lhs.swap(rhs);
    }

    inline void swap(ospf::pointer::UniquePtrLockImpl& lhs, ospf::pointer::UniquePtrLockImpl& rhs)
    {
        lhs.swap(rhs);
    }
#endif
};
