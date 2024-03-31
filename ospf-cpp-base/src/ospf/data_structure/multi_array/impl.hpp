#pragma once

#include <ospf/data_structure/multi_array/shape.hpp>
#include <ospf/data_structure/multi_array/dummy_index.hpp>
#include <ospf/data_structure/multi_array/map_index.hpp>
#include <ospf/data_structure/reference_array.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        template<typename A>
        concept MultiArrayType = requires (A& array)
        {
            { std::declval<typename A::ValueType>() } -> DecayNotSameAs<void>;
            { std::declval<typename A::ShapeType>() } -> multi_array::ShapeType;
            { A::dim } -> DecaySameAs<usize>;
            { array.shape() } -> multi_array::ShapeType;
            { array.get(std::declval<typename A::VectorViewType>()) } -> DecaySameAs<typename A::ValueType>;
            { array.get(std::declval<std::span<const multi_array::dummy_index::DummyIndex, A::dim>>()) } -> DecaySameAs<typename A::ValueType>;
            { array[std::declval<typename A::VectorViewType>()] } -> DecaySameAs<typename A::ValueType>;
            { array[std::declval<std::span<const multi_array::dummy_index::DummyIndex, A::dim>>()] } -> DecaySameAs<typename A::ValueType>;
            { array.begin() } -> std::forward_iterator;
            { array.end() } -> std::forward_iterator;
            { array.rbegin() } -> std::forward_iterator;
            { array.rend() } -> std::forward_iterator;
        };

        namespace multi_array
        {
            using RangeFull = range_bounds::RangeFull;
            using DummyIndex = dummy_index::DummyIndex;
            using MapPlaceHolder = map_index::MapPlaceHolder;
            using MapIndex = map_index::MapIndex;

            template<typename... Args>
            struct IsNormalVector
            {
                static constexpr const bool value = false;
            };

            template<typename T>
            struct IsNormalVector<T>
            {
                static constexpr const bool value = DecaySameAs<T, usize> || std::integral<T>;
            };

            template<typename T, typename... Args>
            struct IsNormalVector<T, Args...>
            {
                static constexpr const bool value = IsNormalVector<T>::value && IsNormalVector<Args...>::value;
            };

            template<typename... Args>
            static constexpr const bool is_normal_vector = IsNormalVector<Args...>::value;

            template<typename... Args>
            struct IsDummyVector
            {
                static constexpr const bool value = false;
            };

            template<typename T>
            struct IsDummyVector<T>
            {
                static constexpr const bool value = dummy_index::DummyIndex::Types::template contains<T>;
            };

            template<typename T, typename... Args>
            struct IsDummyVector<T, Args...>
            {
                static constexpr const bool value = IsDummyVector<T>::value && IsDummyVector<Args...>::value;
            };

            template<typename... Args>
            static constexpr const bool is_dummy_vector = IsDummyVector<Args...>::value;

            template<typename T, typename C, ShapeType S, typename Self>
            class MultiArrayImpl
            {
                OSPF_CRTP_IMPL

            public:
                using ValueType = OriginType<T>;
                using ContainerType = OriginType<C>;
                using ShapeType = OriginType<S>;
                using VectorType = typename ShapeType::VectorType;
                using VectorViewType = typename ShapeType::VectorViewType;

                static constexpr const auto dim = ShapeType::dim;
                using DummyVectorType = std::conditional_t<dim == dynamic_dimension, std::vector<DummyIndex>, std::array<DummyIndex, dim>>;
                using DummyVectorViewType = std::span<const DummyIndex, dim>;

            protected:
                constexpr MultiArrayImpl(void) = default;
            public:
                constexpr MultiArrayImpl(const MultiArrayImpl& ano) = default;
                constexpr MultiArrayImpl(MultiArrayImpl&& ano) noexcept = default;
                constexpr MultiArrayImpl& operator=(const MultiArrayImpl& rhs) = default;
                constexpr MultiArrayImpl& operator=(MultiArrayImpl&& rhs) noexcept = default;
                constexpr ~MultiArrayImpl(void) noexcept = default;

            public:
                inline constexpr LRefType<ContainerType> raw(void) noexcept
                {
                    return Trait::get_container(self());
                }

                inline constexpr CLRefType<ContainerType> raw(void) const noexcept
                {
                    return Trait::get_const_container(self());
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<ValueType>>; }
                inline constexpr const PtrType<ValueType> data(void) noexcept
                {
                    return raw().data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<ValueType>>; }
                inline constexpr const CPtrType<ValueType> data(void) const noexcept
                {
                    return raw().data();
                }

            public:
                inline constexpr LRefType<ValueType> get(const usize index)
                {
                    return Trait::get_value(raw(), index);
                }

                inline constexpr CLRefType<ValueType> get(const usize index) const
                {
                    return Trait::get_const_value(raw(), index);
                }

                inline constexpr LRefType<ValueType> get(ArgCLRefType<VectorViewType> vector)
                {
                    const auto index = shape().index(vector);
                    if (index.is_failed())
                    {
                        throw OSPFException{ std::move(index).err() };
                    }
                    return get(*index);
                }

                inline constexpr CLRefType<ValueType> get(ArgCLRefType<VectorViewType> vector) const
                {
                    const auto index = shape().index(vector);
                    if (index.is_failed())
                    {
                        throw OSPFException{ std::move(index).err() };
                    }
                    return get(*index);
                }

                inline constexpr DynRefArray<ValueType> get(const RangeFull _) const
                {
                    DynRefArray<ValueType> ret;
                    ret.reserve(raw().size());
                    for (const auto& val : raw())
                    {
                        ret.push_back(val);
                    }
                    return ret;
                }

                inline constexpr DynRefArray<ValueType> get(const DummyVectorViewType dummy_vector) const
                {
                    dummy_index::DummyAccessEnumerator<ShapeType> iter{ shape(), dummy_vector };
                    DynRefArray<ValueType> ret;
                    ret.reserve(iter.size());
                    while (iter)
                    {
                        const auto vector = *iter;
                        ret.push_back(get(vector));
                        ++iter;
                    }
                    return ret;
                }

            public:
                // todo: use operator[...] to replace operator(...) in C++23

                template<typename... Args>
                    requires is_normal_vector<Args...>
                        && (dim == dynamic_dimension || dim == VariableTypeList<Args...>::length)
                inline constexpr LRefType<ValueType> operator()(Args&&... args)
                {
                    return get(normal_vector(shape(), std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires is_normal_vector<Args...>
                        && (dim == dynamic_dimension || dim == VariableTypeList<Args...>::length)
                inline constexpr CLRefType<ValueType> operator()(Args&&... args) const
                {
                    return get(normal_vector(shape(), std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires is_dummy_vector<Args...> && !is_normal_vector<Args...>
                        && (dim == dynamic_dimension || dim == VariableTypeList<Args...>::length)
                inline constexpr DynRefArray<ValueType> operator()(Args&&... args) const
                {
                    return get(DummyVectorType{ DummyIndex{ std::forward<Args>(args) }... });
                }

            public:
                inline constexpr LRefType<ValueType> operator[](const VectorViewType vector)
                {
                    return get(vector);
                }

                inline constexpr CLRefType<ValueType> operator[](const VectorViewType vector) const
                {
                    return get(vector);
                }

                inline constexpr LRefType<ValueType> operator[](std::initializer_list<usize> vector)
                {
                    return get(VectorType{ std::move(vector) });
                }

                inline constexpr CLRefType<ValueType> operator[](std::initializer_list<usize> vector) const
                {
                    return get(VectorType{ std::move(vector) });
                }

                inline constexpr DynRefArray<ValueType> operator[](const DummyVectorViewType vector) const
                {
                    return get(vector);
                }

                inline constexpr DynRefArray<ValueType> operator[](std::initializer_list<DummyIndex> vector) const
                {
                    return get(DummyVectorType{ vector });
                }

            public:
                inline constexpr decltype(auto) begin(void) noexcept
                {
                    return raw().begin();
                }

                inline constexpr decltype(auto) begin(void) const noexcept
                {
                    return raw().begin();
                }

                inline constexpr decltype(auto) cbegin(void) const noexcept
                {
                    return raw().cbegin();
                }

                inline constexpr decltype(auto) end(void) noexcept
                {
                    return raw().end();
                }

                inline constexpr decltype(auto) end(void) const noexcept
                {
                    return raw().end();
                }

                inline constexpr decltype(auto) cend(void) const noexcept
                {
                    return raw().cend();
                }

                inline constexpr decltype(auto) rbegin(void) noexcept
                {
                    return raw().rbegin();
                }

                inline constexpr decltype(auto) rbegin(void) const noexcept
                {
                    return raw().rbegin();
                }

                inline constexpr decltype(auto) crbegin(void) const noexcept
                {
                    return raw().crbegin();
                }

                inline constexpr decltype(auto) rend(void) noexcept
                {
                    return raw().rend();
                }

                inline constexpr decltype(auto) rend(void) const noexcept
                {
                    return raw().rend();
                }

                inline constexpr decltype(auto) crend(void) const noexcept
                {
                    return raw().crend();
                }

            public:
                inline constexpr CLRefType<ShapeType> shape(void) const noexcept
                {
                    return Trait::get_shape(self());
                }

                inline constexpr const usize dimension(void) const noexcept
                {
                    return shape().dimension();
                }

                inline constexpr const usize size(void) const noexcept
                {
                    return shape().size();
                }

                inline constexpr const usize max_size(void) const noexcept
                {
                    return raw().max_size();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.reserve(std::declval<usize>()); }
                inline constexpr void reserve(const usize new_capacity)
                {
                    raw().reserve(new_capacity);
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.capacity() } -> DecaySameAs<usize>; }
                inline constexpr const usize capacity(void) const noexcept
                {
                    return raw().capacity();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { container.shrink_to_fit(); }
                inline void shrink_to_fit(void)
                {
                    raw().shrink_to_fit();
                }

            private:
                template<typename... Args>
                    requires is_normal_vector<Args...> && !is_dummy_vector<Args...> && !is_map_vector<Args...>
                        && (dim == dynamic_dimension || dim == VariableTypeList<Args...>::length)
                inline static constexpr RetType<VectorType> normal_vector(const ShapeType& shape, Args&&... args)
                {
                    if constexpr (dim == dynamic_dimension)
                    {
                        if (VariableTypeList<Args...>::length != shape.dimension())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", shape.dimension(), VariableTypeList<Args...>::length) };
                        }
                    }
                    VectorType ret = shape.zero();
                    normal_vector_impl<0_uz>(ret, shape, std::forward<Args>(args)...);
                    return ret;
                }

                template<usize i, typename T, typename... Args>
                inline static constexpr void normal_vector_impl(VectorType& vector, const ShapeType& shape, T&& arg, Args&&... args) noexcept
                {
                    if constexpr (std::signed_integral<T>)
                    {
                        vector[i] = shape.actual_index(i, static_cast<isize>(std::forward<T>(arg)));
                        normal_vector_impl<i + 1_uz>(vector, shape, std::forward<Args>(args)...);
                    }
                    else if constexpr (std::unsigned_integral<T>)
                    {
                        vector[i] = static_cast<usize>(std::forward<T>(arg));
                        normal_vector_impl<i + 1_uz>(vector, shape, std::forward<Args>(args)...);
                    }
                    //else
                    //{
                    //    static_assert(false, "vector dimension mismatched shape.");
                    //}
                }

            protected:
                template<typename Ret, usize vec_dim, usize to_dim>
                    requires (vec_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) map(const ShapeType& shape, const map_index::MapVector<vec_dim, to_dim>& vector)
                {
                    if constexpr (dim == dynamic_dimension)
                    {
                        if (vec_dim != dimension())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", shape.dimension(), vec_dim) };
                        }
                    }

                    const auto map_dimension = map_to_dimension(vector);
                    const auto to_shape = map_to_shape(shape, map_dimension);
                    auto base_vector = base_map_to_vector(shape, vector);

                    Ret ret;
                    ret.reserve(to_shape.size());
                    auto map_vector = to_shape.zero();
                    do
                    {
                        for (usize i{ 0_uz }; i != to_dim; ++i)
                        {
                            base_vector[map_dimension[i]] = DummyIndex{ map_vector[i] };
                        }
                        ret.push_back(get(base_vector));
                    } while (to_shape.next_vector(map_vector));

                    return std::make_pair(std::move(to_shape), std::move(ret));
                }

                template<typename Ret, usize vec_dim, usize to_dim>
                    requires (vec_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) map(const ShapeType& shape, map_index::MapVector<vec_dim, to_dim>&& vector)
                {
                    if constexpr (dim == dynamic_dimension)
                    {
                        if (vec_dim != dimension())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", shape.dimension(), vec_dim) };
                        }
                    }

                    const auto map_dimension = map_to_dimension(vector);
                    const auto to_shape = map_to_shape(shape, map_dimension);
                    auto base_vector = base_map_to_vector(shape, std::move(vector));

                    Ret ret;
                    ret.reserve(to_shape.size());
                    auto map_vector = to_shape.zero();
                    do
                    {
                        for (usize i{ 0_uz }; i != to_dim; ++i)
                        {
                            base_vector[map_dimension[i]] = DummyIndex{ map_vector[i] };
                        }
                        ret.push_back(get(base_vector));
                    } while (to_shape.next_vector(map_vector));

                    return std::make_pair(std::move(to_shape), std::move(ret));
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr CLRefType<ShapeType> get_shape(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_shape);
                        return (self.*get_impl)();
                    }

                    inline static constexpr LRefType<ContainerType> get_container(Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_container);
                        return (self.*get_impl)();
                    }

                    inline static constexpr CLRefType<ContainerType> get_const_container(const Self& self) noexcept
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_container);
                        return (self.*get_impl)();
                    }

                    inline static constexpr LRefType<ValueType> get_value(ContainerType& array, const usize i)
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_value);
                        return (*get_impl)(array, i);
                    }

                    inline static constexpr CLRefType<ValueType> get_const_value(const ContainerType& array, const usize i)
                    {
                        static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_value);
                        return (*get_impl)(array, i);
                    }
                };
            };
        };
    };
};
