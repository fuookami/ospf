#pragma once

#include <ospf/concepts/copy_faster.hpp>
#include <type_traits>

namespace ospf
{
    template<typename T>
    struct TypeFamily
    {
        using Self = std::decay_t<T>;
        using ConstType = std::add_const_t<Self>;

        using ArrayType = Self[];
        using CArrayType = const Self[];

        using PtrType = std::add_pointer_t<Self>;
        using CPtrType = std::add_pointer_t<std::add_const_t<Self>>;

        using LRefType = std::add_lvalue_reference_t<Self>;
        using CLRefType = std::add_lvalue_reference_t<std::add_const_t<Self>>;
        using RRefType = std::add_rvalue_reference_t<Self>;
    };

    template<>
    struct TypeFamily<void>
    {
        using Self = void;
        using ConstType = void;

        using ArrayType = void*;
        using CArrayType = void*;

        using PtrType = void*;
        using CPtrType = void*;

        using LRefType = Self;
        using CLRefType = Self;
        using RRefType = Self;
    };

    template<typename T>
    struct ArgTypeFamily
        : public TypeFamily<T>
    {
        using typename TypeFamily<T>::Self;
        using typename TypeFamily<T>::LRefType;
        using typename TypeFamily<T>::CLRefType;
        using typename TypeFamily<T>::RRefType;

        using ArgLRefType = LRefType;
        using ArgCLRefType = CLRefType;
        using ArgRRefType = RRefType;

        using RetType = Self;
    };

    template<CopyFaster T>
    struct ArgTypeFamily<T>
        : public TypeFamily<T>
    {
        using typename TypeFamily<T>::Self;

        using ArgLRefType = std::add_lvalue_reference_t<Self>;
        using ArgCLRefType = std::add_const_t<Self>;
        using ArgRRefType = std::add_const_t<Self>;

        using RetType = std::add_const_t<Self>;
    };

    template<typename T>
    using OriginType = typename TypeFamily<T>::Self;
    template<typename T>
    using ConstType = typename TypeFamily<T>::ConstType;

    template<typename T>
    using ArrayType = typename TypeFamily<T>::ArrayType;
    template<typename T>
    using CArrayType = typename TypeFamily<T>::CArrayType;

    template<typename T>
    using PtrType = typename TypeFamily<T>::PtrType;
    template<typename T>
    using CPtrType = typename TypeFamily<T>::CPtrType;

    template<typename T>
    using LRefType = typename TypeFamily<T>::LRefType;
    template<typename T>
    using CLRefType = typename TypeFamily<T>::CLRefType;
    template<typename T>
    using RRefType = typename TypeFamily<T>::RRefType;

    template<typename T>
    using ArgLRefType = typename ArgTypeFamily<T>::ArgLRefType;
    template<typename T>
    using ArgCLRefType = typename ArgTypeFamily<T>::ArgCLRefType;
    template<typename T>
    using ArgRRefType = typename ArgTypeFamily<T>::ArgRRefType;

    template<typename T>
    using RetType = typename ArgTypeFamily<T>::RetType;
};
