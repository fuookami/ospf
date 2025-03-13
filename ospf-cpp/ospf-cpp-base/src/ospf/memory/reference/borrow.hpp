#pragma once

#include <ospf/memory/reference/unique_borrow.hpp>

namespace ospf
{
    inline namespace memory
    {
        namespace reference
        {
            template<typename T>
            class Ref<T, ReferenceCategory::Borrow>
                : RefImpl<T, Ref<T, ReferenceCategory::Borrow>>
            {
            private:
                using Impl = RefImpl<T, Ref<T, ReferenceCategory::Borrow>>;

            public:
                using typename Impl::PtrType;
                using typename Impl::CPtrType;
                using typename Impl::RefType;
                using typename Impl::CRefType;

            public:
                Ref(CRefType cref) noexcept
                    : _ptr(cref) {}

            public:
                Ref(const Ref& ano) = default;
                Ref(Ref&& ano) noexcept = default;
                Ref& operator=(const Ref& rhs) = default;
                Ref& operator=(Ref&& rhs) noexcept = default;
                ~Ref(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline RefType OSPF_CRTP_FUNCTION(get_ref)(void) noexcept
                {
                    return *_ptr;
                }

                inline CRefType OSPF_CRTP_FUNCTION(get_cref)(void) const noexcept
                {
                    return *_ptr;
                }

            private:
                Ptr<T> _ptr;
            };
        };
    };
};
